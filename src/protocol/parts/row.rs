use crate::conn_core::AmConnCore;
use crate::protocol::parts::hdb_value::HdbValue;
use crate::protocol::parts::resultset_metadata::ResultSetMetadata;
use crate::HdbResult;

use serde;
use serde_db::de::DeserializableRow;
use std::fmt;
use std::sync::Arc;

/// A single line of a `ResultSet`, consisting of the contained `HdbValue`s and
/// a reference to the metadata.
///
/// `Row` has several methods that support an efficient data transfer into your own data structures.
///
/// You also can access individual values with `row[idx]`, or iterate over the values (with
/// `row.iter()` or `for value in row {...}`).
#[derive(Clone, Debug)]
pub struct Row {
    metadata: Arc<ResultSetMetadata>,
    value_iter: <Vec<HdbValue> as IntoIterator>::IntoIter,
}

impl Row {
    /// Factory for row.
    pub(crate) fn new(metadata: Arc<ResultSetMetadata>, values: Vec<HdbValue>) -> Row {
        Row {
            metadata,
            value_iter: values.into_iter(),
        }
    }

    /// Converts the entire Row into a rust value.
    pub fn try_into<'de, T>(self) -> HdbResult<T>
    where
        T: serde::de::Deserialize<'de>,
    {
        trace!("Row::into_typed()");
        Ok(DeserializableRow::into_typed(self)?)
    }

    /// Removes and returns the next value.
    pub fn next_value(&mut self) -> Option<HdbValue> {
        self.value_iter.next()
    }

    /// Returns the length of the row.
    pub fn len(&self) -> usize {
        trace!("Row::len()");
        self.value_iter.len()
    }

    /// Returns true if the row contains no value.
    pub fn is_empty(&self) -> bool {
        self.value_iter.as_slice().is_empty()
    }

    /// Returns the metadata.
    pub fn metadata(&self) -> &ResultSetMetadata {
        trace!("Row::metadata()");
        &(self.metadata)
    }

    pub(crate) fn number_of_fields(&self) -> usize {
        self.metadata.number_of_fields()
    }

    pub(crate) fn parse(
        md: std::sync::Arc<ResultSetMetadata>,
        am_conn_core: &AmConnCore,
        rdr: &mut std::io::BufRead,
    ) -> HdbResult<Row> {
        let no_of_cols = md.number_of_fields();
        let mut values = Vec::<HdbValue>::new();
        for c in 0..no_of_cols {
            let type_id = md.type_id(c)?;
            let nullable = md.nullable(c)?;
            let scale = md.scale(c)?;
            trace!(
                "Parsing column {}, {}{}",
                c,
                if nullable { "Nullable " } else { "" },
                type_id,
            );
            let value = HdbValue::parse_from_reply(type_id, scale, nullable, am_conn_core, rdr)?;
            values.push(value);
        }
        let row = Row::new(md, values);
        Ok(row)
    }
}

/// Support indexing.
impl std::ops::Index<usize> for Row {
    type Output = HdbValue;
    fn index(&self, idx: usize) -> &HdbValue {
        &self.value_iter.as_slice()[idx]
    }
}

/// Row is an iterator with item = HdbValue.
impl Iterator for Row {
    type Item = HdbValue;
    fn next(&mut self) -> Option<HdbValue> {
        self.next_value()
    }
}

impl fmt::Display for Row {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        for v in self.value_iter.as_slice() {
            write!(fmt, "{}, ", &v)?;
        }
        Ok(())
    }
}

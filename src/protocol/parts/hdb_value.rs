use bigdecimal::BigDecimal;
use protocol::parts::type_id;
use protocol::{cesu8, util};
use types::BLob;
use types::CLob;
use types::DayDate;
use types::LongDate;
use types::SecondDate;
use types::SecondTime;
use types_impl::hdb_decimal::serialize_decimal;
use types_impl::lob::{serialize_blob_header, serialize_clob_header};
use {HdbError, HdbResult};

use byteorder::{LittleEndian, WriteBytesExt};
use serde;
use serde_db::de::DbValue;
use std::fmt;
use std::i16;
use std::io;

const MAX_1_BYTE_LENGTH: u8 = 245;
const MAX_2_BYTE_LENGTH: i16 = i16::MAX;
const LENGTH_INDICATOR_2BYTE: u8 = 246;
const LENGTH_INDICATOR_4BYTE: u8 = 247;
const LENGTH_INDICATOR_NULL: u8 = 255;

/// Enum for all supported database value types.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum HdbValue {
    /// Internally used only. Is swapped in where a real value (any of the
    /// others) is swapped out.
    NOTHING,
    /// Stores an 8-bit unsigned integer.
    /// The minimum value is 0. The maximum value is 255.
    TINYINT(u8),
    /// Stores a 16-bit signed integer.
    /// The minimum value is -32,768. The maximum value is 32,767.
    SMALLINT(i16),
    /// Stores a 32-bit signed integer.
    /// The minimum value is -2,147,483,648. The maximum value is 2,147,483,647.
    INT(i32),
    /// Stores a 64-bit signed integer.
    /// The minimum value is -9,223,372,036,854,775,808.
    /// The maximum value is 9,223,372,036,854,775,807.
    BIGINT(i64),
    /// DECIMAL(p, s) is the SQL standard notation for fixed-point decimal.
    /// "p" specifies precision or the number of total digits
    /// (the sum of whole digits and fractional digits).
    /// "s" denotes scale or the number of fractional digits.
    /// If a column is defined as DECIMAL(5, 4) for example,
    /// the numbers 3.14, 3.1415, 3.141592 are stored in the column as 3.1400,
    /// 3.1415, 3.1415, retaining the specified precision(5) and scale(4).
    ///
    /// Precision p can range from 1 to 38, scale s can range from 0 to p.
    /// If the scale is not specified, it defaults to 0.
    /// If precision and scale are not specified, DECIMAL becomes a
    /// floating-point decimal number. In this case, precision and scale
    /// can vary within the range 1 to 34 for precision and -6,111 to 6,176
    /// for scale, depending on the stored value.
    ///
    /// Examples:
    /// 0.0000001234 (1234E-10) has precision 4 and scale 10.
    /// 1.0000001234 (10000001234E-10) has precision 11 and scale 10.
    /// The value 1234000000 (1234E6) has precision 4 and scale -6.
    DECIMAL(BigDecimal),
    /// Stores a single-precision 32-bit floating-point number.
    REAL(f32),
    /// Stores a double-precision 64-bit floating-point number.
    /// The minimum value is -1.7976931348623157E308, the maximum value is
    /// 1.7976931348623157E308 . The smallest positive DOUBLE value is
    /// 2.2250738585072014E-308 and the largest negative DOUBLE value is
    /// -2.2250738585072014E-308.
    DOUBLE(f64),
    /// Fixed-length character String, only ASCII-7 allowed.
    CHAR(String),
    /// The VARCHAR(n) data type specifies a variable-length character string,
    /// where n indicates the maximum length in bytes and is an integer between
    /// 1 and 5000. If the VARCHAR(n) data type is used in a DML query, for
    /// example CAST (A as VARCHAR(n)), <n> indicates the maximum length of
    /// the string in characters. SAP recommends using VARCHAR with ASCII
    /// characters based strings only. For data containing other
    /// characters, SAP recommends using the NVARCHAR data type instead.
    VARCHAR(String),
    /// Fixed-length character string.
    NCHAR(String),
    /// The NVARCHAR(n) data type specifies a variable-length Unicode character
    /// set string, where <n> indicates the maximum length in characters
    /// and is an integer between 1 and 5000.
    NVARCHAR(String),
    /// The BINARY(n) data type is used to store binary data of a specified
    /// length in bytes, where n indicates the fixed length and is an
    /// integer between 1 and 5000.
    BINARY(Vec<u8>),
    /// The VARBINARY(n) data type is used to store binary data of a specified
    /// maximum length in bytes,
    /// where n indicates the maximum length and is an integer between 1 and 5000.
    VARBINARY(Vec<u8>),
    /// The CLOB data type is used to store a large ASCII character string.
    CLOB(CLob),
    /// The NCLOB data type is used to store a large Unicode string.
    NCLOB(CLob),
    /// The BLOB data type is used to store a large binary string.
    BLOB(BLob),
    /// BOOLEAN stores boolean values, which are TRUE or FALSE.
    BOOLEAN(bool),
    /// The DB returns all Strings as type STRING, independent of the concrete
    /// column type.
    STRING(String),
    /// Likely not used?
    NSTRING(String),
    /// The DB returns all binary values as type BSTRING.
    BSTRING(Vec<u8>),
    /// The SMALLDECIMAL is a floating-point decimal number.
    /// The precision and scale can vary within the range 1~16 for precision
    /// and -369~368 for scale, depending on the stored value.  
    /// SMALLDECIMAL is only supported on the HANA column store.
    /// DECIMAL and SMALLDECIMAL are floating-point types.
    /// For instance, a decimal column can store any of 3.14, 3.1415, 3.141592
    /// whilst maintaining their precision.
    /// DECIMAL(p, s) is the SQL standard notation for fixed-point decimal.
    /// 3.14, 3.1415, 3.141592 are stored in a decimal(5, 4) column as 3.1400,
    /// 3.1415, 3.1415 for example,
    /// retaining the specified precision(5) and scale(4).
    SMALLDECIMAL(BigDecimal),
    /// The TEXT data type enables text search features.
    /// This data type can be defined for column tables, but not for row tables.
    /// This is not a standalone SQL-Type. Selecting a TEXT column yields a
    /// column of type NCLOB.
    TEXT(String),
    /// Similar to TEXT.
    SHORTTEXT(String),
    /// Timestamp with 10^-7 seconds precision, uses eight bytes.
    LONGDATE(LongDate),
    /// TIMESTAMP with second precision.
    SECONDDATE(SecondDate),
    /// DATE.
    DAYDATE(DayDate),
    /// TIME.
    SECONDTIME(SecondTime),
    /// Nullable variant of TINYINT.
    N_TINYINT(Option<u8>),
    /// Nullable variant of SMALLINT.
    N_SMALLINT(Option<i16>),
    /// Nullable variant of INT.
    N_INT(Option<i32>),
    /// Nullable variant of BIGINT.
    N_BIGINT(Option<i64>),
    /// Nullable variant of DECIMAL and DECIMAL(p,s).
    N_DECIMAL(Option<BigDecimal>),
    /// Nullable variant of REAL.
    N_REAL(Option<f32>),
    /// Nullable variant of DOUBLE.
    N_DOUBLE(Option<f64>),
    /// Nullable variant of CHAR.
    N_CHAR(Option<String>),
    /// Nullable variant of VARCHAR.
    N_VARCHAR(Option<String>),
    /// Nullable variant of NCHAR.
    N_NCHAR(Option<String>),
    /// Nullable variant of NVARCHAR.
    N_NVARCHAR(Option<String>),
    /// Nullable variant of BINARY.
    N_BINARY(Option<Vec<u8>>),
    /// Nullable variant of VARBINARY.
    N_VARBINARY(Option<Vec<u8>>),
    /// Nullable variant of CLOB.
    N_CLOB(Option<CLob>),
    /// Nullable variant of NCLOB.
    N_NCLOB(Option<CLob>),
    /// Nullable variant of BLOB.
    N_BLOB(Option<BLob>),
    /// Nullable variant of BOOLEAN.
    N_BOOLEAN(Option<bool>),
    /// Nullable variant of STRING.
    N_STRING(Option<String>),
    /// Nullable variant of NSTRING.
    N_NSTRING(Option<String>),
    /// Nullable variant of BSTRING.
    N_BSTRING(Option<Vec<u8>>),
    /// Nullable variant of SMALLDECIMAL.
    N_SMALLDECIMAL(Option<BigDecimal>),
    /// Nullable variant of TEXT.
    N_TEXT(Option<String>),
    /// Nullable variant of SHORTTEXT.
    N_SHORTTEXT(Option<String>),
    /// Nullable variant of SECONDDATE.
    N_SECONDDATE(Option<SecondDate>),
    /// Nullable variant of DAYDATE.
    N_DAYDATE(Option<DayDate>),
    /// Nullable variant of SECONDTIME.
    N_SECONDTIME(Option<SecondTime>),
    /// Nullable variant of LONGDATE.
    N_LONGDATE(Option<LongDate>),
}

impl HdbValue {
    fn serialize_type_id(&self, w: &mut io::Write) -> HdbResult<bool> {
        let is_null = match *self {
            HdbValue::N_TINYINT(None)
            | HdbValue::N_SMALLINT(None)
            | HdbValue::N_INT(None)
            | HdbValue::N_BIGINT(None)
            | HdbValue::N_REAL(None)
            | HdbValue::N_BOOLEAN(None)
            | HdbValue::N_LONGDATE(None)
            | HdbValue::N_CLOB(None)
            | HdbValue::N_NCLOB(None)
            | HdbValue::N_BLOB(None)
            | HdbValue::N_CHAR(None)
            | HdbValue::N_VARCHAR(None)
            | HdbValue::N_NCHAR(None)
            | HdbValue::N_NVARCHAR(None)
            | HdbValue::N_STRING(None)
            | HdbValue::N_NSTRING(None)
            | HdbValue::N_TEXT(None)
            | HdbValue::N_SHORTTEXT(None)
            | HdbValue::N_BINARY(None)
            | HdbValue::N_VARBINARY(None)
            | HdbValue::N_BSTRING(None) => true,
            _ => false,
        };

        if is_null {
            w.write_u8(self.type_id())?;
        } else {
            w.write_u8(self.type_id() % 128)?;
        }
        Ok(is_null)
    }

    /// hdb protocol uses ids < 128 for non-null values, and ids > 128 for null
    /// values
    fn type_id(&self) -> u8 {
        match *self {
            HdbValue::NOTHING => type_id::NOTHING,
            HdbValue::TINYINT(_) => type_id::TINYINT,
            HdbValue::SMALLINT(_) => type_id::SMALLINT,
            HdbValue::INT(_) => type_id::INT,
            HdbValue::BIGINT(_) => type_id::BIGINT,
            HdbValue::DECIMAL(_) => type_id::DECIMAL,
            HdbValue::REAL(_) => type_id::REAL,
            HdbValue::DOUBLE(_) => type_id::DOUBLE,
            HdbValue::CHAR(_) => type_id::CHAR,
            HdbValue::VARCHAR(_) => type_id::VARCHAR,
            HdbValue::NCHAR(_) => type_id::NCHAR,
            HdbValue::NVARCHAR(_) => type_id::NVARCHAR,
            HdbValue::BINARY(_) => type_id::BINARY,
            HdbValue::VARBINARY(_) => type_id::VARBINARY,
            // HdbValue::TIMESTAMP(_)        => type_id::TIMESTAMP,
            HdbValue::CLOB(_) => type_id::CLOB,
            HdbValue::NCLOB(_) => type_id::NCLOB,
            HdbValue::BLOB(_) => type_id::BLOB,
            HdbValue::BOOLEAN(_) => type_id::BOOLEAN,
            HdbValue::STRING(_) => type_id::STRING,
            HdbValue::NSTRING(_) => type_id::NSTRING,
            HdbValue::BSTRING(_) => type_id::BSTRING,
            HdbValue::SMALLDECIMAL(_) => type_id::SMALLDECIMAL,
            HdbValue::TEXT(_) => type_id::TEXT,
            HdbValue::SHORTTEXT(_) => type_id::SHORTTEXT,
            HdbValue::LONGDATE(_) => type_id::LONGDATE,
            HdbValue::SECONDDATE(_) => type_id::SECONDDATE,
            HdbValue::DAYDATE(_) => type_id::DAYDATE,
            HdbValue::SECONDTIME(_) => type_id::SECONDTIME,
            HdbValue::N_TINYINT(_) => type_id::N_TINYINT,
            HdbValue::N_SMALLINT(_) => type_id::N_SMALLINT,
            HdbValue::N_INT(_) => type_id::N_INT,
            HdbValue::N_BIGINT(_) => type_id::N_BIGINT,
            HdbValue::N_DECIMAL(_) => type_id::N_DECIMAL,
            HdbValue::N_REAL(_) => type_id::N_REAL,
            HdbValue::N_DOUBLE(_) => type_id::N_DOUBLE,
            HdbValue::N_CHAR(_) => type_id::N_CHAR,
            HdbValue::N_VARCHAR(_) => type_id::N_VARCHAR,
            HdbValue::N_NCHAR(_) => type_id::N_NCHAR,
            HdbValue::N_NVARCHAR(_) => type_id::N_NVARCHAR,
            HdbValue::N_BINARY(_) => type_id::N_BINARY,
            HdbValue::N_VARBINARY(_) => type_id::N_VARBINARY,
            // HdbValue::N_TIMESTAMP(_)       => type_id::N_TIMESTAMP,
            HdbValue::N_CLOB(_) => type_id::N_CLOB,
            HdbValue::N_NCLOB(_) => type_id::N_NCLOB,
            HdbValue::N_BLOB(_) => type_id::N_BLOB,
            HdbValue::N_BOOLEAN(_) => type_id::N_BOOLEAN,
            HdbValue::N_STRING(_) => type_id::N_STRING,
            HdbValue::N_NSTRING(_) => type_id::N_NSTRING,
            HdbValue::N_BSTRING(_) => type_id::N_BSTRING,
            HdbValue::N_SMALLDECIMAL(_) => type_id::N_SMALLDECIMAL,
            HdbValue::N_TEXT(_) => type_id::N_TEXT,
            HdbValue::N_SHORTTEXT(_) => type_id::N_SHORTTEXT,
            HdbValue::N_LONGDATE(_) => type_id::N_LONGDATE,
            HdbValue::N_SECONDDATE(_) => type_id::N_SECONDDATE,
            HdbValue::N_DAYDATE(_) => type_id::N_DAYDATE,
            HdbValue::N_SECONDTIME(_) => type_id::N_SECONDTIME,
        }
    }

    /// Deserialize into a rust type
    pub fn try_into<'x, T: serde::Deserialize<'x>>(self) -> Result<T, HdbError> {
        Ok(DbValue::into_typed(self)?)
    }
}

pub fn serialize(tv: &HdbValue, data_pos: &mut i32, w: &mut io::Write) -> HdbResult<()> {
    if !tv.serialize_type_id(w)? {
        match *tv {
            HdbValue::NOTHING => {
                return Err(HdbError::Impl(
                    "Can't send HdbValue::NOTHING to Database".to_string(),
                ))
            }
            HdbValue::TINYINT(u) | HdbValue::N_TINYINT(Some(u)) => w.write_u8(u)?,

            HdbValue::SMALLINT(i) | HdbValue::N_SMALLINT(Some(i)) => {
                w.write_i16::<LittleEndian>(i)?
            }

            HdbValue::INT(i) | HdbValue::N_INT(Some(i)) => w.write_i32::<LittleEndian>(i)?,

            HdbValue::BIGINT(i) | HdbValue::N_BIGINT(Some(i)) => w.write_i64::<LittleEndian>(i)?,

            HdbValue::DECIMAL(ref bigdec)
            | HdbValue::N_DECIMAL(Some(ref bigdec))
            | HdbValue::SMALLDECIMAL(ref bigdec)
            | HdbValue::N_SMALLDECIMAL(Some(ref bigdec)) => serialize_decimal(bigdec, w)?,

            HdbValue::REAL(f) | HdbValue::N_REAL(Some(f)) => w.write_f32::<LittleEndian>(f)?,

            HdbValue::DOUBLE(f) | HdbValue::N_DOUBLE(Some(f)) => w.write_f64::<LittleEndian>(f)?,

            HdbValue::BOOLEAN(true) | HdbValue::N_BOOLEAN(Some(true)) => w.write_u8(1)?,
            HdbValue::BOOLEAN(false) | HdbValue::N_BOOLEAN(Some(false)) => w.write_u8(0)?,

            HdbValue::LONGDATE(ref ld) | HdbValue::N_LONGDATE(Some(ref ld)) => {
                w.write_i64::<LittleEndian>(*ld.ref_raw())?
            }

            HdbValue::SECONDDATE(ref sd) | HdbValue::N_SECONDDATE(Some(ref sd)) => {
                w.write_i64::<LittleEndian>(*sd.ref_raw())?
            }

            HdbValue::DAYDATE(ref dd) | HdbValue::N_DAYDATE(Some(ref dd)) => {
                w.write_i32::<LittleEndian>(*dd.ref_raw())?
            }
            HdbValue::SECONDTIME(ref st) | HdbValue::N_SECONDTIME(Some(ref st)) => {
                w.write_u32::<LittleEndian>(*st.ref_raw())?
            }

            HdbValue::CLOB(ref clob)
            | HdbValue::N_CLOB(Some(ref clob))
            | HdbValue::NCLOB(ref clob)
            | HdbValue::N_NCLOB(Some(ref clob)) => serialize_clob_header(clob.len()?, data_pos, w)?,

            HdbValue::BLOB(ref blob) | HdbValue::N_BLOB(Some(ref blob)) => {
                serialize_blob_header(blob.len_alldata(), data_pos, w)?
            }

            HdbValue::STRING(ref s)
            | HdbValue::NSTRING(ref s)
            | HdbValue::TEXT(ref s)
            | HdbValue::SHORTTEXT(ref s)
            | HdbValue::N_STRING(Some(ref s))
            | HdbValue::N_NSTRING(Some(ref s))
            | HdbValue::N_TEXT(Some(ref s))
            | HdbValue::N_SHORTTEXT(Some(ref s)) => serialize_length_and_string(s, w)?,

            HdbValue::BINARY(ref v)
            | HdbValue::VARBINARY(ref v)
            | HdbValue::BSTRING(ref v)
            | HdbValue::N_BINARY(Some(ref v))
            | HdbValue::N_VARBINARY(Some(ref v))
            | HdbValue::N_BSTRING(Some(ref v)) => serialize_length_and_bytes(v, w)?,

            HdbValue::N_TINYINT(None)
            | HdbValue::N_SMALLINT(None)
            | HdbValue::N_INT(None)
            | HdbValue::N_BIGINT(None)
            | HdbValue::N_DECIMAL(None)
            | HdbValue::N_SMALLDECIMAL(None)
            | HdbValue::N_REAL(None)
            | HdbValue::N_DOUBLE(None)
            | HdbValue::N_BOOLEAN(None)
            | HdbValue::N_LONGDATE(None)
            | HdbValue::N_SECONDDATE(None)
            | HdbValue::N_DAYDATE(None)
            | HdbValue::N_SECONDTIME(None)
            | HdbValue::N_STRING(None)
            | HdbValue::N_NSTRING(None)
            | HdbValue::N_TEXT(None)
            | HdbValue::N_SHORTTEXT(None)
            | HdbValue::N_CLOB(None)
            | HdbValue::N_NCLOB(None)
            | HdbValue::N_BLOB(None)
            | HdbValue::N_BINARY(None)
            | HdbValue::N_VARBINARY(None)
            | HdbValue::N_BSTRING(None) => {}

            HdbValue::CHAR(_)
            | HdbValue::N_CHAR(_)
            | HdbValue::NCHAR(_)
            | HdbValue::N_NCHAR(_)
            | HdbValue::VARCHAR(_)
            | HdbValue::N_VARCHAR(_)
            | HdbValue::NVARCHAR(_)
            | HdbValue::N_NVARCHAR(_) => {
                return Err(HdbError::Impl(format!(
                    "HdbValue::serialize() not implemented for type code {}",
                    tv.type_id()
                )))
            }
        }
    }
    Ok(())
}

// is used to calculate the argument size (in serialize)
pub fn size(tv: &HdbValue) -> HdbResult<usize> {
    Ok(1 + match *tv {
        HdbValue::NOTHING => {
            return Err(HdbError::Impl(
                "Can't send HdbValue::NOTHING to Database".to_string(),
            ))
        }
        HdbValue::BOOLEAN(_)
        | HdbValue::N_BOOLEAN(Some(_))
        | HdbValue::TINYINT(_)
        | HdbValue::N_TINYINT(Some(_)) => 1,

        HdbValue::SMALLINT(_) | HdbValue::N_SMALLINT(Some(_)) => 2,

        HdbValue::DECIMAL(_)
        | HdbValue::N_DECIMAL(Some(_))
        | HdbValue::SMALLDECIMAL(_)
        | HdbValue::N_SMALLDECIMAL(Some(_)) => 16,

        HdbValue::INT(_)
        | HdbValue::N_INT(Some(_))
        | HdbValue::REAL(_)
        | HdbValue::N_REAL(Some(_))
        | HdbValue::DAYDATE(_)
        | HdbValue::N_DAYDATE(Some(_))
        | HdbValue::SECONDTIME(_)
        | HdbValue::N_SECONDTIME(Some(_)) => 4,

        HdbValue::BIGINT(_)
        | HdbValue::N_BIGINT(Some(_))
        | HdbValue::DOUBLE(_)
        | HdbValue::N_DOUBLE(Some(_))
        | HdbValue::LONGDATE(_)
        | HdbValue::N_LONGDATE(Some(_))
        | HdbValue::SECONDDATE(_)
        | HdbValue::N_SECONDDATE(Some(_)) => 8,

        HdbValue::CLOB(ref clob)
        | HdbValue::N_CLOB(Some(ref clob))
        | HdbValue::NCLOB(ref clob)
        | HdbValue::N_NCLOB(Some(ref clob)) => 9 + clob.len()?,

        HdbValue::BLOB(ref blob) | HdbValue::N_BLOB(Some(ref blob)) => 9 + blob.len_alldata(),

        HdbValue::STRING(ref s)
        | HdbValue::N_STRING(Some(ref s))
        | HdbValue::NSTRING(ref s)
        | HdbValue::N_NSTRING(Some(ref s))
        | HdbValue::TEXT(ref s)
        | HdbValue::N_TEXT(Some(ref s))
        | HdbValue::SHORTTEXT(ref s)
        | HdbValue::N_SHORTTEXT(Some(ref s)) => string_length(s),

        HdbValue::BINARY(ref v)
        | HdbValue::N_BINARY(Some(ref v))
        | HdbValue::VARBINARY(ref v)
        | HdbValue::N_VARBINARY(Some(ref v))
        | HdbValue::BSTRING(ref v)
        | HdbValue::N_BSTRING(Some(ref v)) => v.len() + 2,

        HdbValue::N_TINYINT(None)
        | HdbValue::N_SMALLINT(None)
        | HdbValue::N_INT(None)
        | HdbValue::N_BIGINT(None)
        | HdbValue::N_DECIMAL(None)
        | HdbValue::N_SMALLDECIMAL(None)
        | HdbValue::N_REAL(None)
        | HdbValue::N_DOUBLE(None)
        | HdbValue::N_BOOLEAN(None)
        | HdbValue::N_LONGDATE(None)
        | HdbValue::N_SECONDDATE(None)
        | HdbValue::N_DAYDATE(None)
        | HdbValue::N_SECONDTIME(None)
        | HdbValue::N_CLOB(None)
        | HdbValue::N_NCLOB(None)
        | HdbValue::N_BLOB(None)
        | HdbValue::N_BINARY(None)
        | HdbValue::N_VARBINARY(None)
        | HdbValue::N_BSTRING(None)
        | HdbValue::N_STRING(None)
        | HdbValue::N_NSTRING(None)
        | HdbValue::N_TEXT(None)
        | HdbValue::N_SHORTTEXT(None) => 0,

        HdbValue::CHAR(_)
        | HdbValue::VARCHAR(_)
        | HdbValue::NCHAR(_)
        | HdbValue::NVARCHAR(_)
        | HdbValue::N_CHAR(_)
        | HdbValue::N_VARCHAR(_)
        | HdbValue::N_NCHAR(_)
        | HdbValue::N_NVARCHAR(_) => {
            return Err(HdbError::Impl(format!(
                "HdbValue::size() not implemented for type code {}",
                tv.type_id()
            )))
        }
    })
}

pub fn string_length(s: &str) -> usize {
    match cesu8::cesu8_length(s) {
        clen if clen <= MAX_1_BYTE_LENGTH as usize => 1 + clen,
        clen if clen <= MAX_2_BYTE_LENGTH as usize => 3 + clen,
        clen => 5 + clen,
    }
}

pub fn serialize_length_and_string(s: &str, w: &mut io::Write) -> HdbResult<()> {
    serialize_length_and_bytes(&cesu8::string_to_cesu8(s), w)
}

fn serialize_length_and_bytes(v: &[u8], w: &mut io::Write) -> HdbResult<()> {
    match v.len() {
        l if l <= MAX_1_BYTE_LENGTH as usize => {
            w.write_u8(l as u8)?; // B1           LENGTH OF VALUE
        }
        l if l <= MAX_2_BYTE_LENGTH as usize => {
            w.write_u8(LENGTH_INDICATOR_2BYTE)?; // B1           246
            w.write_i16::<LittleEndian>(l as i16)?; // I2           LENGTH OF VALUE
        }
        l => {
            w.write_u8(LENGTH_INDICATOR_4BYTE)?; // B1           247
            w.write_i32::<LittleEndian>(l as i32)?; // I4           LENGTH OF VALUE
        }
    }
    util::serialize_bytes(v, w) // B variable   VALUE BYTES
}

impl fmt::Display for HdbValue {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            HdbValue::NOTHING => write!(fmt, "Value already swapped out"),
            HdbValue::TINYINT(value) | HdbValue::N_TINYINT(Some(value)) => write!(fmt, "{}", value),
            HdbValue::SMALLINT(value) | HdbValue::N_SMALLINT(Some(value)) => {
                write!(fmt, "{}", value)
            }
            HdbValue::INT(value) | HdbValue::N_INT(Some(value)) => write!(fmt, "{}", value),
            HdbValue::BIGINT(value) | HdbValue::N_BIGINT(Some(value)) => write!(fmt, "{}", value),
            HdbValue::DECIMAL(ref value)
            | HdbValue::N_DECIMAL(Some(ref value))
            | HdbValue::SMALLDECIMAL(ref value)
            | HdbValue::N_SMALLDECIMAL(Some(ref value)) => write!(fmt, "{}", value),
            HdbValue::REAL(value) | HdbValue::N_REAL(Some(value)) => write!(fmt, "{}", value),
            HdbValue::DOUBLE(value) | HdbValue::N_DOUBLE(Some(value)) => write!(fmt, "{}", value),
            HdbValue::CHAR(ref value)
            | HdbValue::N_CHAR(Some(ref value))
            | HdbValue::VARCHAR(ref value)
            | HdbValue::N_VARCHAR(Some(ref value))
            | HdbValue::NCHAR(ref value)
            | HdbValue::N_NCHAR(Some(ref value))
            | HdbValue::NVARCHAR(ref value)
            | HdbValue::N_NVARCHAR(Some(ref value))
            | HdbValue::STRING(ref value)
            | HdbValue::N_STRING(Some(ref value))
            | HdbValue::NSTRING(ref value)
            | HdbValue::N_NSTRING(Some(ref value))
            | HdbValue::TEXT(ref value)
            | HdbValue::N_TEXT(Some(ref value))
            | HdbValue::SHORTTEXT(ref value)
            | HdbValue::N_SHORTTEXT(Some(ref value)) => write!(fmt, "{}", value),
            HdbValue::BINARY(_) | HdbValue::N_BINARY(Some(_)) => write!(fmt, "<BINARY>"),
            HdbValue::VARBINARY(ref vec) | HdbValue::N_VARBINARY(Some(ref vec)) => {
                write!(fmt, "<VARBINARY length = {}>", vec.len())
            }
            HdbValue::CLOB(_) | HdbValue::N_CLOB(Some(_)) => write!(fmt, "<CLOB>"),
            HdbValue::NCLOB(_) | HdbValue::N_NCLOB(Some(_)) => write!(fmt, "<NCLOB>"),
            HdbValue::BLOB(ref blob) | HdbValue::N_BLOB(Some(ref blob)) => write!(
                fmt,
                "<BLOB length = {}, read = {}>",
                blob.len_alldata(),
                blob.len_readdata()
            ),
            HdbValue::BOOLEAN(value) | HdbValue::N_BOOLEAN(Some(value)) => write!(fmt, "{}", value),
            HdbValue::BSTRING(_) | HdbValue::N_BSTRING(Some(_)) => write!(fmt, "<BSTRING>"),
            HdbValue::LONGDATE(ref value) | HdbValue::N_LONGDATE(Some(ref value)) => {
                write!(fmt, "{}", value)
            }
            HdbValue::SECONDDATE(ref value) | HdbValue::N_SECONDDATE(Some(ref value)) => {
                write!(fmt, "{}", value)
            }
            HdbValue::DAYDATE(ref value) | HdbValue::N_DAYDATE(Some(ref value)) => {
                write!(fmt, "{}", value)
            }
            HdbValue::SECONDTIME(ref value) | HdbValue::N_SECONDTIME(Some(ref value)) => {
                write!(fmt, "{}", value)
            }

            HdbValue::N_TINYINT(None)
            | HdbValue::N_SMALLINT(None)
            | HdbValue::N_INT(None)
            | HdbValue::N_BIGINT(None)
            | HdbValue::N_DECIMAL(None)
            | HdbValue::N_SMALLDECIMAL(None)
            | HdbValue::N_REAL(None)
            | HdbValue::N_DOUBLE(None)
            | HdbValue::N_CHAR(None)
            | HdbValue::N_VARCHAR(None)
            | HdbValue::N_NCHAR(None)
            | HdbValue::N_NVARCHAR(None)
            | HdbValue::N_BINARY(None)
            | HdbValue::N_VARBINARY(None)
            | HdbValue::N_CLOB(None)
            | HdbValue::N_NCLOB(None)
            | HdbValue::N_BLOB(None)
            | HdbValue::N_BOOLEAN(None)
            | HdbValue::N_STRING(None)
            | HdbValue::N_NSTRING(None)
            | HdbValue::N_BSTRING(None)
            | HdbValue::N_TEXT(None)
            | HdbValue::N_SHORTTEXT(None)
            | HdbValue::N_LONGDATE(None)
            | HdbValue::N_SECONDDATE(None)
            | HdbValue::N_DAYDATE(None)
            | HdbValue::N_SECONDTIME(None) => write!(fmt, "<NULL>"),
        }
    }
}

pub mod factory {
    use super::HdbValue;
    use conn_core::AmConnCore;
    use protocol::{cesu8, util};
    use types_impl::daydate::{parse_daydate, parse_nullable_daydate};
    use types_impl::hdb_decimal::{parse_decimal, parse_nullable_decimal};
    use types_impl::lob::{parse_blob, parse_clob, parse_nullable_blob, parse_nullable_clob};
    use types_impl::longdate::{parse_longdate, parse_nullable_longdate};
    use types_impl::seconddate::{parse_nullable_seconddate, parse_seconddate};
    use types_impl::secondtime::{parse_nullable_secondtime, parse_secondtime};
    use {HdbError, HdbResult};

    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io;
    use std::iter::repeat;
    use std::{u32, u64};

    pub fn parse_from_reply(
        p_typecode: u8,
        nullable: bool,
        am_conn_core: &AmConnCore,
        rdr: &mut io::BufRead,
    ) -> HdbResult<HdbValue> {
        // here p_typecode is always < 127
        // the flag nullable from the metadata governs our behavior:
        // if it is true, we return types with typecode above 128, which use
        // Option<type>, if it is false, we return types with the original
        // typecode, which use plain values
        let typecode = p_typecode + if nullable { 128 } else { 0 };
        match typecode {
            1 => Ok(HdbValue::TINYINT({
                ind_not_null(rdr)?;
                rdr.read_u8()?
            })),
            2 => Ok(HdbValue::SMALLINT({
                ind_not_null(rdr)?;
                rdr.read_i16::<LittleEndian>()?
            })),
            3 => Ok(HdbValue::INT({
                ind_not_null(rdr)?;
                rdr.read_i32::<LittleEndian>()?
            })),
            4 => Ok(HdbValue::BIGINT({
                ind_not_null(rdr)?;
                rdr.read_i64::<LittleEndian>()?
            })),
            5 => Ok(HdbValue::DECIMAL(parse_decimal(rdr)?)),
            6 => Ok(HdbValue::REAL(parse_real(rdr)?)),
            7 => Ok(HdbValue::DOUBLE(parse_double(rdr)?)),
            8 => Ok(HdbValue::CHAR(parse_string(rdr)?)),
            9 => Ok(HdbValue::VARCHAR(parse_string(rdr)?)),
            10 => Ok(HdbValue::NCHAR(parse_string(rdr)?)),
            11 => Ok(HdbValue::NVARCHAR(parse_string(rdr)?)),
            12 => Ok(HdbValue::BINARY(parse_binary(rdr)?)),
            13 => Ok(HdbValue::VARBINARY(parse_binary(rdr)?)),
            // 16 => Ok(HdbValue::TIMESTAMP(
            25 => Ok(HdbValue::CLOB(parse_clob(am_conn_core, rdr)?)),
            26 => Ok(HdbValue::NCLOB(parse_clob(am_conn_core, rdr)?)),
            27 => Ok(HdbValue::BLOB(parse_blob(am_conn_core, rdr)?)),
            28 => Ok(HdbValue::BOOLEAN(rdr.read_u8()? > 0)),
            29 => Ok(HdbValue::STRING(parse_string(rdr)?)),
            30 => Ok(HdbValue::NSTRING(parse_string(rdr)?)),
            33 => Ok(HdbValue::BSTRING(parse_binary(rdr)?)),
            // 47 => Ok(HdbValue::SMALLDECIMAL(
            51 => Ok(HdbValue::TEXT(parse_string(rdr)?)),
            52 => Ok(HdbValue::SHORTTEXT(parse_string(rdr)?)),
            61 => Ok(HdbValue::LONGDATE(parse_longdate(rdr)?)),
            62 => Ok(HdbValue::SECONDDATE(parse_seconddate(rdr)?)),
            63 => Ok(HdbValue::DAYDATE(parse_daydate(rdr)?)),
            64 => Ok(HdbValue::SECONDTIME(parse_secondtime(rdr)?)),
            129 => Ok(HdbValue::N_TINYINT(if ind_null(rdr)? {
                None
            } else {
                Some(rdr.read_u8()?)
            })),

            130 => Ok(HdbValue::N_SMALLINT(if ind_null(rdr)? {
                None
            } else {
                Some(rdr.read_i16::<LittleEndian>()?)
            })),
            131 => Ok(HdbValue::N_INT(if ind_null(rdr)? {
                None
            } else {
                Some(rdr.read_i32::<LittleEndian>()?)
            })),
            132 => Ok(HdbValue::N_BIGINT(if ind_null(rdr)? {
                None
            } else {
                Some(rdr.read_i64::<LittleEndian>()?)
            })),
            133 => Ok(HdbValue::N_DECIMAL(parse_nullable_decimal(rdr)?)),
            134 => Ok(HdbValue::N_REAL(parse_nullable_real(rdr)?)),
            135 => Ok(HdbValue::N_DOUBLE(parse_nullable_double(rdr)?)),
            136 => Ok(HdbValue::N_CHAR(parse_nullable_string(rdr)?)),
            137 => Ok(HdbValue::N_VARCHAR(parse_nullable_string(rdr)?)),
            138 => Ok(HdbValue::N_NCHAR(parse_nullable_string(rdr)?)),
            139 => Ok(HdbValue::N_NVARCHAR(parse_nullable_string(rdr)?)),
            140 => Ok(HdbValue::N_BINARY(parse_nullable_binary(rdr)?)),
            141 => Ok(HdbValue::N_VARBINARY(parse_nullable_binary(rdr)?)),
            // 144 => Ok(HdbValue::N_TIMESTAMP(
            153 => Ok(HdbValue::N_CLOB(parse_nullable_clob(am_conn_core, rdr)?)),
            154 => Ok(HdbValue::N_NCLOB(parse_nullable_clob(am_conn_core, rdr)?)),
            155 => Ok(HdbValue::N_BLOB(parse_nullable_blob(am_conn_core, rdr)?)),
            156 => Ok(HdbValue::N_BOOLEAN(if ind_null(rdr)? {
                None
            } else {
                Some(rdr.read_u8()? > 0)
            })),
            157 => Ok(HdbValue::N_STRING(parse_nullable_string(rdr)?)),
            158 => Ok(HdbValue::N_NSTRING(parse_nullable_string(rdr)?)),
            161 => Ok(HdbValue::N_BSTRING(parse_nullable_binary(rdr)?)),
            // 175 => Ok(HdbValue::N_SMALLDECIMAL(
            179 => Ok(HdbValue::N_TEXT(parse_nullable_string(rdr)?)),
            180 => Ok(HdbValue::N_SHORTTEXT(parse_nullable_string(rdr)?)),
            189 => Ok(HdbValue::N_LONGDATE(parse_nullable_longdate(rdr)?)),
            190 => Ok(HdbValue::N_SECONDDATE(parse_nullable_seconddate(rdr)?)),
            191 => Ok(HdbValue::N_DAYDATE(parse_nullable_daydate(rdr)?)),
            192 => Ok(HdbValue::N_SECONDTIME(parse_nullable_secondtime(rdr)?)),
            _ => Err(HdbError::Impl(format!(
                "HdbValue::parse_from_reply() not implemented for type code {}",
                typecode
            ))),
        }
    }

    // reads the nullindicator and returns Ok(true) if it has value 0 or Ok(false)
    // otherwise
    fn ind_null(rdr: &mut io::BufRead) -> HdbResult<bool> {
        Ok(rdr.read_u8()? == 0)
    }

    // reads the nullindicator and throws an error if it has value 0
    fn ind_not_null(rdr: &mut io::BufRead) -> HdbResult<()> {
        if ind_null(rdr)? {
            Err(HdbError::Impl(
                "null value returned for not-null column".to_owned(),
            ))
        } else {
            Ok(())
        }
    }

    fn parse_real(rdr: &mut io::BufRead) -> HdbResult<f32> {
        let mut vec: Vec<u8> = repeat(0u8).take(4).collect();
        rdr.read_exact(&mut vec[..])?;
        let mut r = io::Cursor::new(&vec);
        let tmp = r.read_u32::<LittleEndian>()?;
        match tmp {
            u32::MAX => Err(HdbError::Impl(
                "Unexpected NULL Value in parse_real()".to_owned(),
            )),
            _ => {
                r.set_position(0);
                Ok(r.read_f32::<LittleEndian>()?)
            }
        }
    }

    fn parse_nullable_real(rdr: &mut io::BufRead) -> HdbResult<Option<f32>> {
        let mut vec: Vec<u8> = repeat(0u8).take(4).collect();
        rdr.read_exact(&mut vec[..])?;
        let mut r = io::Cursor::new(&vec);
        let tmp = r.read_u32::<LittleEndian>()?;
        match tmp {
            u32::MAX => Ok(None),
            _ => {
                r.set_position(0);
                Ok(Some(r.read_f32::<LittleEndian>()?))
            }
        }
    }

    fn parse_double(rdr: &mut io::BufRead) -> HdbResult<f64> {
        let mut vec: Vec<u8> = repeat(0u8).take(8).collect();
        rdr.read_exact(&mut vec[..])?;
        let mut r = io::Cursor::new(&vec);
        let tmp = r.read_u64::<LittleEndian>()?;
        match tmp {
            u64::MAX => Err(HdbError::Impl(
                "Unexpected NULL Value in parse_double()".to_owned(),
            )),
            _ => {
                r.set_position(0);
                Ok(r.read_f64::<LittleEndian>()?)
            }
        }
    }

    fn parse_nullable_double(rdr: &mut io::BufRead) -> HdbResult<Option<f64>> {
        let mut vec: Vec<u8> = repeat(0u8).take(8).collect();
        rdr.read_exact(&mut vec[..])?;
        let mut r = io::Cursor::new(&vec);
        let tmp = r.read_u64::<LittleEndian>()?;
        match tmp {
            u64::MAX => Ok(None),
            _ => {
                r.set_position(0);
                Ok(Some(r.read_f64::<LittleEndian>()?))
            }
        }
    }

    // ----- STRINGS and BINARIES
    // ----------------------------------------------------------------
    pub fn parse_string(rdr: &mut io::BufRead) -> HdbResult<String> {
        match cesu8::cesu8_to_string(&parse_binary(rdr)?) {
            Ok(s) => Ok(s),
            Err(e) => {
                error!("cesu-8 problem occured in hdb_value:parse_string()");
                Err(e)
            }
        }
    }

    fn parse_binary(rdr: &mut io::BufRead) -> HdbResult<Vec<u8>> {
        let l8 = rdr.read_u8()?; // B1
        let len = match l8 {
            l if l <= super::MAX_1_BYTE_LENGTH => l8 as usize,
            super::LENGTH_INDICATOR_2BYTE => rdr.read_i16::<LittleEndian>()? as usize, // I2
            super::LENGTH_INDICATOR_4BYTE => rdr.read_i32::<LittleEndian>()? as usize, // I4
            l => {
                return Err(HdbError::Impl(format!(
                    "Invalid value in length indicator: {}",
                    l
                )));
            }
        };
        let result = util::parse_bytes(len, rdr)?; // B (varying)
        trace!("parse_binary(): read_bytes = {:?}, len = {}", result, len);
        Ok(result)
    }

    fn parse_nullable_string(rdr: &mut io::BufRead) -> HdbResult<Option<String>> {
        match parse_nullable_binary(rdr)? {
            Some(vec) => match cesu8::cesu8_to_string(&vec) {
                Ok(s) => Ok(Some(s)),
                Err(_) => Err(HdbError::Impl(
                    "cesu-8 problem occured in hdb_value:parse_string()".to_owned(),
                )),
            },
            None => Ok(None),
        }
    }

    fn parse_nullable_binary(rdr: &mut io::BufRead) -> HdbResult<Option<Vec<u8>>> {
        let l8 = rdr.read_u8()?; // B1
        let len = match l8 {
            l if l <= super::MAX_1_BYTE_LENGTH => l8 as usize,
            super::LENGTH_INDICATOR_2BYTE => rdr.read_i16::<LittleEndian>()? as usize, // I2
            super::LENGTH_INDICATOR_4BYTE => rdr.read_i32::<LittleEndian>()? as usize, // I4
            super::LENGTH_INDICATOR_NULL => return Ok(None),
            l => {
                return Err(HdbError::Impl(format!(
                    "Invalid value in length indicator: {}",
                    l
                )))
            }
        };
        let result = util::parse_bytes(len, rdr)?;
        trace!("parse_nullable_binary(): read_bytes = {:?}", result);
        Ok(Some(result)) // B (varying)
    }
}

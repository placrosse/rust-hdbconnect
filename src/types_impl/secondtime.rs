use byteorder::{LittleEndian, ReadBytesExt};
use std::cmp;
use std::fmt;
use std::io;
use {HdbError, HdbResult};

const NULL_REPRESENTATION: i32 = 86_402;

const MINUTE_FACTOR: u32 = 60;
const HOUR_FACTOR: u32 = 3_600;

/// Implementation of HANA's `SecondTime`.
///
/// The type is used internally to implement serialization to the wire.
///
/// HANA allows input of empty strings, they are mapped to 0, all other legal values are mapped to
/// Hours * 60*60 + Minutes * 60 + Seconds  + 1 < 86400.
///
/// When reading, we treat 0 and 1 as "00:00:00".
#[derive(Clone, Debug)]
pub struct SecondTime(u32);

impl fmt::Display for SecondTime {
    // The format chosen supports the conversion to chrono types.
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let (hour, minute, second) = self.as_hms();
        write!(fmt, "{:02}:{:02}:{:02}", hour, minute, second)
    }
}

impl cmp::PartialEq<SecondTime> for SecondTime {
    fn eq(&self, other: &SecondTime) -> bool {
        self.0 == other.0
    }
}

impl SecondTime {
    #[doc(hidden)]
    pub fn new(raw: i32) -> SecondTime {
        assert!(raw < NULL_REPRESENTATION && raw >= 0);
        SecondTime(raw as u32)
    }
    #[doc(hidden)]
    pub fn ref_raw(&self) -> &u32 {
        &self.0
    }

    /// Factory method for SecondTime with all fields.
    pub fn from_hms(hour: u32, minute: u32, second: u32) -> Result<SecondTime, &'static str> {
        if hour > 23 || minute > 59 || second > 59 {
            Err("illegal value of hour, minute or second")
        } else {
            Ok(SecondTime(
                hour * HOUR_FACTOR + minute * MINUTE_FACTOR + second + 1,
            ))
        }
    }

    /// Convert into tuple of "elements".
    pub fn as_hms(&self) -> (u32, u32, u32) {
        let mut second = if self.0 == 0 { 0 } else { self.0 - 1 };
        let hour = second / HOUR_FACTOR;
        second -= HOUR_FACTOR * hour;
        let minute = second / MINUTE_FACTOR;
        second -= MINUTE_FACTOR * minute;

        (hour, minute, second)
    }
}

pub fn parse_secondtime(rdr: &mut io::BufRead) -> HdbResult<SecondTime> {
    let st = rdr.read_i32::<LittleEndian>()?;
    match st {
        NULL_REPRESENTATION => Err(HdbError::Impl(
            "Null value found for non-null secondtime column".to_owned(),
        )),
        _ => Ok(SecondTime::new(st)),
    }
}

pub fn parse_nullable_secondtime(rdr: &mut io::BufRead) -> HdbResult<Option<SecondTime>> {
    let st = rdr.read_i32::<LittleEndian>()?;
    match st {
        NULL_REPRESENTATION => Ok(None),
        _ => Ok(Some(SecondTime::new(st))),
    }
}
use crate::hdb_error::HdbResult;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io;

#[derive(Debug)]
pub struct ReadLobRequest {
    locator_id: u64,
    offset: u64,
    length_to_read: i32,
}
impl ReadLobRequest {
    pub fn new(locator_id: u64, offset: u64, length_to_read: i32) -> ReadLobRequest {
        ReadLobRequest {
            locator_id,
            offset,
            length_to_read,
        }
    }
    pub fn emit<T: io::Write>(&self, w: &mut T) -> HdbResult<()> {
        trace!("read_lob_request::emit() {:?}", self);
        w.write_u64::<LittleEndian>(self.locator_id)?;
        w.write_u64::<LittleEndian>(self.offset)?;
        w.write_i32::<LittleEndian>(self.length_to_read)?;
        w.write_u32::<LittleEndian>(0_u32)?; // FILLER
        Ok(())
    }
    pub fn size(&self) -> usize {
        24
    }
}

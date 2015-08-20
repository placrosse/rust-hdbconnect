use super::bufread::*;
use super::segment::*;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{BufRead, Error, ErrorKind, Result, Write};
use std::net::TcpStream;


const BUFFER_SIZE: u32 = 130000;
const MESSAGE_HEADER_SIZE: u32 = 32;



// Packets having the same sequence number belong to one request/response pair.
#[derive(Debug)]
pub struct Message {
    session_id: i64,
    packet_seq_number: i32,
    segments: Vec<Segment>,
}

/// Serialize to byte stream
impl Message {
    pub fn send(&mut self, stream: &mut TcpStream) -> Result<Message> {
        trace!("Entering DbStream::send_and_receive()");

        try!(self.serialize(stream));
        debug!("send_and_receive: request data successfully sent");

        let mut rdr = BufReader::new(stream);
        Message::try_to_parse(&mut rdr)
    }


    fn serialize(&self, w: &mut Write) -> Result<()> {
        trace!("Entering Message::serialize()");

        let varpart_size = self.varpart_size();
        let total_size = MESSAGE_HEADER_SIZE + varpart_size;
        trace!("Writing Message with total size {}", total_size);
        let remaining_bufsize = BUFFER_SIZE - MESSAGE_HEADER_SIZE;
        let mut b = Vec::<u8>::with_capacity(total_size as usize);

        // MESSAGE HEADER
        try!(b.write_i64::<LittleEndian>(self.session_id));             // I8
        try!(b.write_i32::<LittleEndian>(self.packet_seq_number));      // I4
        try!(b.write_u32::<LittleEndian>(varpart_size));                // UI4
        try!(b.write_u32::<LittleEndian>(remaining_bufsize));           // UI4
        try!(b.write_i16::<LittleEndian>(self.segments.len() as i16));  // I2
        try!(b.write_i8(0));                                            // I1    unused
        for _ in 0..9 { try!(b.write_u8(0)); }                          // B[9]  unused

        // SEGMENTS
        let mut osr = (0u32, 1i16, remaining_bufsize); // offset, segment_no, remaining_bufsize
        for ref segment in &self.segments {
            osr = try!(segment.encode(osr.0, osr.1, osr.2, &mut b));
        }

        try!(w.write(&b));
        w.flush()
    }

    pub fn new(session_id: i64, packet_seq_number: i32) -> Message {
        Message {
            session_id: session_id,
            packet_seq_number: packet_seq_number,
            segments: Vec::<Segment>::new(),
        }
    }

    pub fn push(&mut self, segment: Segment){
        self.segments.push(segment);
    }

    /// Length in bytes of the variable part of the message, i.e. total message without the header
    fn varpart_size(&self) -> u32 {
        let mut len = 0;
        for seg in &self.segments {
            len += seg.size() as u32;
        }
        trace!("varpart_size = {}",len);
        len
    }

    ///
    fn try_to_parse(rdr: &mut BufReader<&mut TcpStream>) -> Result<Message> {
        trace!("Entering Message::try_to_parse()");

        loop {
            trace!("looping in Message::try_to_parse()");
            match try_to_parse_msg_header(rdr) {
                Ok(ParseResponse::MsgHdr(msg, varpart_size, remaining_bufsize, no_of_segs)) => {
                    // FIXME parse segments
                    return Ok(msg);
                },
                Ok(ParseResponse::Incomplete) => {
                    trace!("get_init_response: got Incomplete from try_to_parse()");
                },
                Err(e) => return Err(Error::from(e)),
            }
            match rdr.read_into_buf() {
                Ok(0) if rdr.get_buf().is_empty() => {
                    return Err(Error::new(ErrorKind::Other, "Connection closed"));
                },
                Ok(0) => return Err(Error::new(ErrorKind::Other, "Response is bigger than expected")), // ???
                Ok(_) => (),
                Err(e) => return Err(Error::from(e))
            }
        }
    }
}


///
fn try_to_parse_msg_header(rdr: &mut BufReader<&mut TcpStream>) -> Result<ParseResponse> {
    trace!("Entering Message::try_to_parse_msg_header()");

    let l = rdr.get_buf().len();
    if  l >= (MESSAGE_HEADER_SIZE as usize) {
        // MESSAGE HEADER: 32 bytes
        let session_id = try!(rdr.read_i64::<LittleEndian>());          // I8
        let packet_seq_number = try!(rdr.read_i32::<LittleEndian>());   // I4
        let varpart_size = try!(rdr.read_u32::<LittleEndian>());        // UI4
        let remaining_bufsize = try!(rdr.read_u32::<LittleEndian>());   // UI4
        let no_of_segs = try!(rdr.read_i16::<LittleEndian>());          // I2
        rdr.consume(MESSAGE_HEADER_SIZE as usize); // ignore the unused last 10 bytes (I1 + B[9])
        debug!("try_to_parse_msg_header() returns Ok");
        Ok(ParseResponse::MsgHdr(
            Message::new(session_id,packet_seq_number),
            varpart_size,
            remaining_bufsize,
            no_of_segs))
    } else {
        trace!("try_to_parse_msg_header() got only {} bytes", l);
        Ok(ParseResponse::Incomplete)
    }
}

enum ParseResponse {
    MsgHdr(Message,u32,u32,i16),
    Incomplete,
}
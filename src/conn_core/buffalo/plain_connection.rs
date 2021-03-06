use crate::conn_core::connect_params::ConnectParams;
use std::cell::RefCell;
use std::io;
use std::net::TcpStream;

#[derive(Debug)]
pub struct PlainConnection {
    params: ConnectParams,
    reader: RefCell<io::BufReader<TcpStream>>,
    writer: RefCell<io::BufWriter<TcpStream>>,
}

impl PlainConnection {
    /// Returns an initialized plain tcp connection
    pub fn try_new(params: ConnectParams) -> io::Result<(PlainConnection)> {
        let tcpstream = TcpStream::connect(params.addr())?;
        Ok(PlainConnection {
            params,
            writer: RefCell::new(io::BufWriter::new(tcpstream.try_clone()?)),
            reader: RefCell::new(io::BufReader::new(tcpstream)),
        })
    }

    pub fn writer(&self) -> &RefCell<io::BufWriter<TcpStream>> {
        &self.writer
    }

    pub fn reader(&self) -> &RefCell<io::BufReader<TcpStream>> {
        &self.reader
    }

    #[allow(dead_code)]
    pub fn reconnect(&self) -> io::Result<()> {
        let tcpstream = TcpStream::connect(self.params.addr())?;
        self.writer
            .replace(io::BufWriter::new(tcpstream.try_clone()?));
        self.reader.replace(io::BufReader::new(tcpstream));
        Ok(())
    }
}

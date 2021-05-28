use drpc::conn;
use drpc::traits::*;
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> Result<()> {
    let enc = NopEncoding::new();
    let socket = TcpStream::connect("localhost:7355")?;
    let mut tr = TCPTransport::new(socket);
    let mut conn = conn::Conn::new(&mut tr);

    let out = conn.invoke("/service.Service/foo".as_bytes(), &enc, &vec![8, 128, 10])?;
    println!("{:?}", &out);

    let stream = conn.stream("/service.Service/foo".as_bytes())?;
    stream.send(&enc, &vec![8, 10])?;
    println!("{:?}", stream.recv(&enc)?);
    stream.close()?;

    let out = conn.invoke("/service.Service/foo".as_bytes(), &enc, &vec![8, 10])?;
    println!("{:?}", &out);

    Ok(())
}

//

struct TCPTransport(TcpStream);

impl TCPTransport {
    fn new(socket: TcpStream) -> TCPTransport {
        TCPTransport(socket)
    }
}

impl Transport for TCPTransport {
    fn write(self: &mut Self, buf: &[u8]) -> Result<usize> {
        Ok(self.0.write(buf)?)
    }

    fn read(self: &mut Self, buf: &mut [u8]) -> Result<usize> {
        Ok(self.0.read(buf)?)
    }

    fn close(self: &mut Self) -> Result<()> {
        Ok(self.0.shutdown(std::net::Shutdown::Both)?)
    }
}

//

struct NopEncoding;

impl NopEncoding {
    fn new() -> NopEncoding {
        NopEncoding {}
    }
}

impl Encoding<Vec<u8>, Vec<u8>> for NopEncoding {
    fn marshal(self: &Self, msg: &Vec<u8>) -> Result<Vec<u8>> {
        Ok(msg.clone())
    }

    fn unmarshal(self: &Self, buf: &[u8]) -> Result<Vec<u8>> {
        Ok(buf.to_vec())
    }
}

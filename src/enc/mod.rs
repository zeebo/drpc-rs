pub type Error = Box<dyn std::error::Error>;

pub type Result<T> = std::result::Result<T, Error>;

pub trait Marshal<In> {
    fn marshal(msg: &In, buf: &mut Vec<u8>) -> Result<()>;
}

pub trait Unmarshal<Out> {
    fn unmarshal(buf: &[u8], out: &mut Out) -> Result<()>;
}

impl Marshal<Vec<u8>> for () {
    fn marshal(msg: &Vec<u8>, buf: &mut Vec<u8>) -> Result<()> {
        *buf = msg.clone();
        Ok(())
    }
}

impl Unmarshal<Vec<u8>> for () {
    fn unmarshal(buf: &[u8], out: &mut Vec<u8>) -> Result<()> {
        *out = buf.to_owned();
        Ok(())
    }
}

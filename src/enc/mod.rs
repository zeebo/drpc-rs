pub trait Marshal<In> {
    fn marshal(msg: &In, buf: &mut Vec<u8>) -> crate::Result<()>;
}

pub trait Unmarshal<Out> {
    fn unmarshal(buf: &[u8], out: &mut Out) -> crate::Result<()>;
}

impl Marshal<Vec<u8>> for () {
    fn marshal(msg: &Vec<u8>, buf: &mut Vec<u8>) -> crate::Result<()> {
        *buf = msg.clone();
        Ok(())
    }
}

impl Unmarshal<Vec<u8>> for () {
    fn unmarshal(buf: &[u8], out: &mut Vec<u8>) -> crate::Result<()> {
        *out = buf.to_owned();
        Ok(())
    }
}

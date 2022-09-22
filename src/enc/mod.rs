pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

pub trait Marshal: Sync + Send {
    fn marshal(&self, buf: &mut Vec<u8>) -> Result<()>;
}

pub trait Unmarshal: Send {
    fn unmarshal(&mut self, buf: &[u8]) -> Result<()>;
}

impl Marshal for Vec<u8> {
    fn marshal(&self, buf: &mut Vec<u8>) -> Result<()> {
        *buf = self.clone();
        Ok(())
    }
}

impl Unmarshal for Vec<u8> {
    fn unmarshal(&mut self, buf: &[u8]) -> Result<()> {
        *self = buf.to_owned();
        Ok(())
    }
}

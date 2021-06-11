pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;

pub trait Encoding<In, Out> {
    fn marshal(self: &Self, msg: &In) -> Result<Vec<u8>>;
    fn unmarshal(self: &Self, buf: &[u8]) -> Result<Out>;
}

pub trait Transport {
    fn write(self: &mut Self, buf: &[u8]) -> Result<usize>;
    fn read(self: &mut Self, buf: &mut [u8]) -> Result<usize>;
    fn close(self: &mut Self) -> Result<()>;
}

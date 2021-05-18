pub trait Transport {
    type Error;

    fn write(self: &mut Self, buf: &[u8]) -> Option<Self::Error>;
    fn read(self: &mut Self, buf: &[u8]) -> (usize, Option<Self::Error>);
    fn close(self: &mut Self) -> Option<Self::Error>;
}

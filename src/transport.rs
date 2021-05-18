pub trait Transport {
    type Error;

    fn write(self: &mut Self, buf: &[u8]) -> Option<Self::Error>;
    fn read(self: &mut Self, buf: &mut [u8]) -> (usize, Option<Self::Error>);
    fn close(self: &mut Self) -> Option<Self::Error>;
}

impl<'a> Transport for &'a mut Vec<u8> {
    type Error = ();

    fn write(self: &mut Self, buf: &[u8]) -> Option<Self::Error> {
        self.extend_from_slice(buf);
        None
    }

    fn read(self: &mut Self, buf: &mut [u8]) -> (usize, Option<Self::Error>) {
        let n = std::cmp::min(buf.len(), self.len());
        buf[0..n].copy_from_slice(&self[0..n]);
        self.drain(0..n);
        (n, None)
    }

    fn close(self: &mut Self) -> Option<Self::Error> {
        None
    }
}

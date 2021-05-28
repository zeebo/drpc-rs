pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub trait Encoding<In, Out> {
    fn marshal(self: &Self, msg: &In) -> Result<Vec<u8>>;
    fn unmarshal(self: &Self, buf: &[u8]) -> Result<Out>;
}

pub trait Transport {
    fn write(self: &mut Self, buf: &[u8]) -> Result<usize>;
    fn read(self: &mut Self, buf: &mut [u8]) -> Result<usize>;
    fn close(self: &mut Self) -> Result<()>;
}

// pub trait Conn<Tr: Transport> {
//     fn close(self: &mut Self) -> Result<()>;
//     fn closed(self: &mut Self) -> bool;
//     fn transport(self: &mut Self) -> &mut Tr;

//     fn invoke<In, Out, Enc: Encoding<In, Out>>(
//         self: &mut Self,
//         rpc: &[u8],
//         enc: &Enc,
//         input: &In,
//     ) -> Result<Out>;

//     fn stream<In, Out, St: Stream<In, Out>>(self: &mut Self, rpc: &[u8]) -> Result<&mut St>;
// }

// pub trait Stream<In, Out> {
//     fn close_send(self: &mut Self) -> Result<()>;
//     fn close(self: &mut Self) -> Result<()>;
//     fn error(self: &mut Self, msg: &str) -> Result<()>;

//     fn send<Enc: Encoding<In, Out>>(self: &mut Self, enc: &Enc, input: &In) -> Result<()>;
//     fn recv<Enc: Encoding<In, Out>>(self: &mut Self, enc: &Enc) -> Result<Out>;
// }

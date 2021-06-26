pub mod frame;
pub mod id;
pub mod packet;
pub mod split;
pub mod transport;
pub mod varint;

pub trait Transport: std::io::Read + std::io::Write {}

impl<T> Transport for T where T: std::io::Read + std::io::Write {}

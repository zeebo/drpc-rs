use crate::wire::id;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Kind {
    Invoke,
    Message,
    Error,
    Close,
    CloseSend,
    InvokeMetadata,
    Other(u8),
}

impl From<Kind> for u8 {
    fn from(kind: Kind) -> u8 {
        match kind {
            Kind::Invoke => 1,
            Kind::Message => 2,
            Kind::Error => 3,
            Kind::Close => 5,
            Kind::CloseSend => 6,
            Kind::InvokeMetadata => 7,
            Kind::Other(x) => x,
        }
    }
}

impl From<u8> for Kind {
    fn from(x: u8) -> Kind {
        match x {
            1 => Kind::Invoke,
            2 => Kind::Message,
            3 => Kind::Error,
            5 => Kind::Close,
            6 => Kind::CloseSend,
            7 => Kind::InvokeMetadata,
            _ => Kind::Other(x),
        }
    }
}

impl Default for Kind {
    fn default() -> Kind {
        Kind::Other(0)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Packet {
    pub data: Vec<u8>,
    pub id: id::ID,
    pub kind: Kind,
}

use super::id;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Kind {
    Invoke = 1,
    Message = 2,
    Error = 3,
    Close = 5,
    CloseSend = 6,
    InvokeMetadata = 7,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Packet<'a> {
    pub data: &'a [u8],
    pub id: id::ID,
    pub kind: Kind,
}

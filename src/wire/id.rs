#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default, Clone, Copy)]
pub struct ID {
    pub stream: u64,
    pub message: u64,
}

impl ID {
    pub fn new(sid: u64, mid: u64) -> Self {
        ID {
            stream: sid,
            message: mid,
        }
    }
}

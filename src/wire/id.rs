#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default, Clone, Copy)]
pub struct ID {
    pub stream: u64,
    pub message: u64,
}

impl ID {
    pub fn new(stream: u64, message: u64) -> Self {
        ID { stream, message }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default, Clone, Copy)]
pub struct ID {
    pub stream: u64,
    pub message: u64,
}

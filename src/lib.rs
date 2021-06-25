#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

pub type Error = Box<dyn std::error::Error>;

pub type Result<T> = std::result::Result<T, Error>;

pub mod conn;
pub mod enc;
pub mod stream;
pub mod utils;
pub mod wire;

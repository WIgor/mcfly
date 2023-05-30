#[derive(Debug, PartialEq)]
pub enum Error {
    EOF,
}

pub const DEFAULT_BUF_SIZE: usize = 1 << 10;

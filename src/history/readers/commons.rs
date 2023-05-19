#[derive(Debug, PartialEq)]
pub enum Error {
    EOF,
    ERR(String),
}
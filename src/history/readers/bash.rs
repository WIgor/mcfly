use rev_lines::RevLines;
use std::io::{BufReader, Read, Seek};
use std::iter::{Iterator};
use crate::history::readers::commons::Error;

pub struct BashHistoryReader<R: Read + Seek> {
    reader: RevLines<R>,
}

impl<R: Read + Seek> Iterator for BashHistoryReader<R> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_command() {
            Ok(s) => Some(s),
            Err(Error::EOF) => None,
        }
    }
}

impl<R: Read + Seek> BashHistoryReader<R> {
    pub fn from_bufreader(buffer: BufReader<R>) -> BashHistoryReader<R> {
        BashHistoryReader::<R>::from_revlines(RevLines::new(buffer).unwrap())
    }

    fn from_revlines(rev_lines: RevLines<R>) -> BashHistoryReader<R> {
        BashHistoryReader {
            reader: rev_lines,
        }
    }

    fn read_command(&mut self) -> Result<String, Error> {
        match self.reader.next() {
            Some(line) => return Ok(line),
            None => Err(Error::EOF),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};
    use crate::history::readers::commons::Error;
    use crate::history::readers::bash::BashHistoryReader;

    #[test]
    fn lines_parsing() {
        let s = b"git
gcc test
vim .zsh
vim .zshrc
ls";
        let mut reader =
            BashHistoryReader::from_bufreader(BufReader::new(Cursor::new(s)));
        assert_eq!(reader.read_command(), Ok("ls".to_string()));
        assert_eq!(reader.read_command(), Ok("vim .zshrc".to_string()));
        assert_eq!(reader.read_command(), Ok("vim .zsh".to_string()));
        assert_eq!(reader.read_command(), Ok("gcc test".to_string()));
        assert_eq!(reader.read_command(), Ok("git".to_string()));
        assert_eq!(reader.read_command(), Err(Error::EOF));
    }
}
use regex::Regex;
use rev_lines::RevLines;
use std::io::{BufReader, Read, Seek};
use std::iter::{Iterator};
use crate::history::readers::commons::Error;

const DEFAULT_BUF_SIZE: usize = 1 << 12;
const ZSH_META_CHAR: u8 = 0x83;

pub struct ZshHistoryReader<R: Read + Seek> {
    reader: RevLines<R>,
    zsh_command_start: Regex,
    command_line: String,
}

impl<R: Read + Seek> Iterator for ZshHistoryReader<R> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_command() {
            Ok(s) => Some(s),
            Err(Error::EOF) => None,
        }
    }
}

impl<R: Read + Seek> ZshHistoryReader<R> {
    pub fn from_bufreader(buffer: BufReader<R>) -> ZshHistoryReader<R> {
        ZshHistoryReader::<R>::from_revlines(RevLines::new(buffer).unwrap())
    }

    fn from_revlines(rev_lines: RevLines<R>) -> ZshHistoryReader<R> {
        ZshHistoryReader {
            reader: rev_lines,
            zsh_command_start: Regex::new(r"^: \d+:\d+;.*").unwrap(),
            command_line: String::with_capacity(DEFAULT_BUF_SIZE),
        }
    }

    fn fix_meta_char(in_line: &str) -> String {
        let buffer = in_line.as_bytes();
        let mut result = Vec::<u8>::with_capacity(buffer.len());
        let mut prev_meta = false;
        for i in 0..buffer.len() {
            let mut current = buffer[i];
            if current == ZSH_META_CHAR {
                prev_meta = true;
                // Change previous byte and skip metachar.
                continue;
            }
            if prev_meta {
                current ^= 32;
            }
            result.push(current);
            prev_meta = false;
        }
        String::from_utf8_lossy(result.as_slice()).to_string()
    }

    fn read_command(&mut self) -> Result<String, Error> {
        match self.reader.next() {
            Some(line) => {
                let fixed = ZshHistoryReader::<R>::fix_meta_char(line.as_str());
                if self.command_line.is_empty() {
                    self.command_line = fixed
                } else {
                    self.command_line = format!("{}\n{}", fixed, self.command_line);
                }

                if self.zsh_command_start.is_match(self.command_line.as_str()) {
                    let result = self.command_line.clone();
                    self.command_line.clear();
                    return Ok(result);
                }
                self.read_command()
            }
            None => Err(Error::EOF),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};
    use crate::history::readers::zsh::Error;
    use crate::history::readers::zsh::ZshHistoryReader;

    #[test]
    fn lines_parsing() {
        let s = b": 1681548611:0;git
: 1681548612:0;gcc \\
test
: 1681548613:0;vim .zsh
: 1681548614:0;vim .zshrc
: 1681548615:0;ls";
        let mut reader =
            ZshHistoryReader::from_bufreader(BufReader::new(Cursor::new(s)));
        assert_eq!(reader.read_command(), Ok(": 1681548615:0;ls".to_string()));
        assert_eq!(reader.read_command(), Ok(": 1681548614:0;vim .zshrc".to_string()));
        assert_eq!(reader.read_command(), Ok(": 1681548613:0;vim .zsh".to_string()));
        assert_eq!(reader.read_command(), Ok(": 1681548612:0;gcc \\\ntest".to_string()));
        assert_eq!(reader.read_command(), Ok(": 1681548611:0;git".to_string()));
        assert_eq!(reader.read_command(), Err(Error::EOF));
    }

    #[test]
    fn replace_meta_char() {
        let s =
            b": 1681739174:0;echo \xE5\xAD\x83\xB7\xE4\xB8\xB2\xE6\xB8\xAC\xE8\xA9\xA6";
        let mut reader =
            ZshHistoryReader::from_bufreader(BufReader::new(Cursor::new(s)));
        assert_eq!(reader.read_command(),
                   Ok(String::from_utf8_lossy(
                       b": 1681739174:0;echo \xE5\xAD\x97\xE4\xB8\xB2\xE6\xB8\xAC\xE8\xA9\xA6")
                       .to_string()));
    }
}
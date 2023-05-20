use regex::Regex;
use rev_lines::RevLines;
use std::io::{BufRead, BufReader, Read, Seek, Cursor};
use std::fs::File;
use std::path::Path;
use std::iter::Iterator;
use crate::history::readers::commons::Error;

const DEFAULT_BUF_SIZE: usize = 1 << 12;
const ZSH_META_CHAR:† u8 = 0x83;
const END_LINE: u8 = b'\n';

pub struct ZshHistoryReader<R: Read + Seek> {
    reader: RevLines<BufReader<R>>,
    zsh_command_start: Regex,
    command_line: String,
}

impl<R: Read + Seek> Iterator for ZshHistoryReader<R> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_command() {
            Ok(s) => Some(s),
            Err(Error::EOF) => None,
            Err(e) => panic!("ZshHistoryReader.read_command {:?}", e)
        }
    }
}

impl<R: Read + Seek> ZshHistoryReader<R> {
    pub fn from_file(path: &Path) -> ZshHistoryReader<R> {
        ZshHistoryReader::from_reader(
            Box::new(File::open(path)
                .unwrap_or_else(|_| panic!("McFly error: {:?} file not found", &path))))
    }

    pub fn from_cursor(cursor: Cursor<u8>) -> ZshHistoryReader<R> {
        ZshHistoryReader {
            reader: RevLines::new(BufReader::new(cursor)).unwrap(),
            zsh_command_start: Regex::new(r"^: \d+:\d+;.*$").unwrap(),
            command_line: String::with_capacity(DEFAULT_BUF_SIZE),
        }
    }

    fn read_line(&mut self) -> Result<String, Error> {
        let mut buffer = Vec::with_capacity(DEFAULT_BUF_SIZE);
        let read_res = self.reader.read_until(END_LINE, &mut buffer);
        let mut prev_meta = false;
        match read_res {
            Ok(size) if size == 0 => Err(Error::EOF),
            Ok(size) => {
                let mut line = Vec::<u8>::with_capacity(size);
                for i in 0..size {
                    let mut current = buffer[i];
                    if current == ZSH_META_CHAR {
                        prev_meta = true;
                        // Change previous byte and skip metachar.
                        continue;
                    }
                    if current == END_LINE {
                        // Skip end line.
                        continue;
                    }
                    if prev_meta {
                        current ^= 32;
                    }
                    line.push(current);
                    prev_meta = false;
                }
                Ok(String::from_utf8_lossy(line.as_slice()).to_string())
            }
            Err(e) => {
                Err(Error::ERR(format!("ERROR {}:{} {}", file!(), line!(), e)))
            }
        }
    }

    fn read_command(&mut self) -> Result<String, Error> {
        match self.read_line() {
            Ok(line) => {
                if self.zsh_command_start.is_match(line.as_str()) {
                    return if self.command_line.is_empty() {
                        self.command_line = line;
                        self.read_command()
                    } else {
                        let result = self.command_line.clone();
                        self.command_line = line;
                        Ok(result)
                    };
                }

                self.command_line.push_str("\n");
                self.command_line.push_str(line.as_str());
                self.read_command()
            },
            Err(Error::EOF) if self.command_line.is_empty() => Err(Error::EOF),
            Err(Error::EOF) => {
                let result = self.command_line.clone();
                self.command_line.clear();
                Ok(result)
            },
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use crate::history::readers::zsh::Error;
    use crate::history::readers::zsh::ZshHistoryReader;

    #[test]
    fn lines_parsing() {
        let s = b": 1681548618:0;git
: 1681548618:0;gcc \\
test
: 1681548618:0;vim .zsh
: 1681548618:0;vim .zshrc
: 1681548618:0;ls";
        let mut reader = ZshHistoryReader::from_reader(Box::new(Cursor::new(s)));
        assert_eq!(reader.read_command(), Ok(": 1681548618:0;git".to_string()));
        assert_eq!(reader.read_command(), Ok(": 1681548618:0;gcc \\\ntest".to_string()));
        assert_eq!(reader.read_command(), Ok(": 1681548618:0;vim .zsh".to_string()));
        assert_eq!(reader.read_command(), Ok(": 1681548618:0;vim .zshrc".to_string()));
        assert_eq!(reader.read_command(), Ok(": 1681548618:0;ls".to_string()));
        assert_eq!(reader.read_command(), Err(Error::EOF));
    }

    #[test]
    fn replace_meta_char() {
        let s =
            b": 1681739174:0;echo \xE5\xAD\x83\xB7\xE4\xB8\xB2\xE6\xB8\xAC\xE8\xA9\xA6";
        let mut reader =
            ZshHistoryReader::from_reader(Box::new(Cursor::new(s)));
        assert_eq!(reader.read_command(),
                   Ok(String::from_utf8_lossy(
                       b": 1681739174:0;echo \xE5\xAD\x97\xE4\xB8\xB2\xE6\xB8\xAC\xE8\xA9\xA6")
                       .to_string()));
    }
}
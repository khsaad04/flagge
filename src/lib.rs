use std::ffi::{OsStr, OsString};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[derive(Debug)]
pub struct Lexer {
    argv: Vec<OsString>,
    cursor: usize,
    offset: usize,
}

#[derive(Debug)]
pub enum Token<'a> {
    ShortFlag(char),
    LongFlag(&'a str),
    Value(OsString),
}

// TODO: Impl `Command` parser with builder pattern

#[derive(Debug)]
pub struct Error {
    ctx: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ctx)
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self { ctx: value }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(value: std::str::Utf8Error) -> Self {
        Self {
            ctx: value.to_string(),
        }
    }
}

impl Lexer {
    pub fn from(argv: impl Iterator<Item = OsString>) -> Self {
        Self {
            argv: argv.collect::<Vec<OsString>>(),
            cursor: 1,
            offset: 0,
        }
    }

    pub fn from_env() -> Self {
        Self::from(std::env::args_os())
    }

    pub fn starts_with_program_name(&mut self, b: bool) {
        if b && self.cursor == 0 && self.offset == 0 {
            self.cursor = 1;
        } else if !b && self.cursor == 1 && self.offset == 0 {
            self.cursor = 0;
        }
    }

    pub fn next_token(&mut self) -> Result<Option<Token>, Error> {
        if self.cursor >= self.argv.len() {
            return Ok(None);
        }

        let current_arg = self.argv[self.cursor].as_bytes();
        if current_arg.starts_with(b"--") {
            let stripped_arg = &current_arg[2..];
            if let Some(pos) = stripped_arg.iter().position(|x| *x == b'=') {
                if pos != 0 {
                    self.offset = pos + 1;
                    self.cursor += 1;
                    match std::str::from_utf8(&stripped_arg[..pos]) {
                        Ok(val) => return Ok(Some(Token::LongFlag(val))),
                        Err(err) => return Err(err.into()),
                    }
                }
            }
            self.cursor += 1;
            match std::str::from_utf8(stripped_arg) {
                Ok(val) => Ok(Some(Token::LongFlag(val))),
                Err(err) => Err(err.into()),
            }
        } else if current_arg.starts_with(b"-") {
            let stripped_arg = &current_arg[1..];
            let stripped_arg_utf8 = OsStr::from_bytes(stripped_arg).to_string_lossy();

            let offset = self.offset;
            if let Some(pos) = stripped_arg.iter().position(|x| *x == b'=') {
                if pos == self.offset + 1 {
                    self.offset += 1;
                }
            }
            if stripped_arg_utf8.chars().count() > self.offset + 1 {
                self.offset += 1;
            } else {
                self.cursor += 1;
                self.offset = 0;
            }

            if stripped_arg_utf8.chars().nth(offset).unwrap() == '�' {
                Err(format!(
                    "Invalid unicode character in {}",
                    String::from_utf8_lossy(current_arg)
                )
                .into())
            } else {
                Ok(Some(Token::ShortFlag(
                    stripped_arg_utf8.chars().nth(offset).unwrap(),
                )))
            }
        } else {
            Ok(Some(Token::Value(OsStr::from_bytes(current_arg).into())))
        }
    }

    pub fn get_value(&mut self) -> Option<OsString> {
        if self.cursor >= self.argv.len() {
            return None;
        }

        let current_arg = self.argv[self.cursor].as_bytes();
        if !current_arg.starts_with(b"-") {
            self.cursor += 1;
            Some(OsStr::from_bytes(current_arg).into())
        } else if current_arg.starts_with(b"--") && self.offset > 0 {
            let stripped_arg = &current_arg[2..];
            if stripped_arg.is_empty() {
                return None;
            }
            let offset = self.offset;
            self.cursor += 1;
            self.offset = 0;
            Some(OsStr::from_bytes(&stripped_arg[offset..]).into())
        } else if current_arg.starts_with(b"-") && self.offset > 0 {
            let stripped_arg = &current_arg[1..];
            let offset = self.offset;
            self.cursor += 1;
            self.offset = 0;
            Some(OsStr::from_bytes(&stripped_arg[offset..]).into())
        } else {
            None
        }
    }

    pub fn finished(&self) -> bool {
        self.cursor >= self.argv.len()
    }
}

// TODO: Write unit tests

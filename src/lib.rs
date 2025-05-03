use std::ffi::{OsStr, OsString};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[derive(Debug)]
pub struct Lexer {
    argv: Vec<OsString>,
    index: usize,
    cursor: usize,
}

#[derive(Debug)]
pub enum Token<'a> {
    ShortFlag(char),
    LongFlag(&'a str),
    Value(OsString),
}

// TODO: Implement a Command parser with builder pattern

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

impl Lexer {
    pub fn from(argv: impl Iterator<Item = OsString>) -> Self {
        Self {
            argv: argv.collect::<Vec<OsString>>(),
            index: 1,
            cursor: 0,
        }
    }

    pub fn from_env() -> Self {
        Self::from(std::env::args_os())
    }

    pub fn starts_with_program_name(&mut self, b: bool) {
        if b && self.index == 0 && self.cursor == 0 {
            self.index = 1;
        } else if !b && self.index == 1 && self.cursor == 0 {
            self.index = 0;
        }
    }

    pub fn next_token(&mut self) -> Result<Option<Token>, Error> {
        if self.index >= self.argv.len() {
            return Ok(None);
        }

        let current_arg = self.argv[self.index].as_bytes();
        if current_arg.starts_with(b"--") {
            let stripped_arg = &current_arg[2..];
            if stripped_arg.is_empty() {
                return Ok(None);
            }
            if let Some(pos) = stripped_arg.iter().position(|x| *x == b'=') {
                if pos != 0 {
                    self.cursor = pos + 1;
                    self.index += 1;
                    match std::str::from_utf8(&stripped_arg[..pos]) {
                        Ok(val) => return Ok(Some(Token::LongFlag(val))),
                        Err(err) => {
                            return Err(format!(
                                "Invalid unicode character(s) in argument {}: {err}",
                                String::from_utf8_lossy(current_arg)
                            )
                            .into());
                        }
                    }
                }
            }

            match std::str::from_utf8(stripped_arg) {
                Ok(val) => {
                    self.index += 1;
                    Ok(Some(Token::LongFlag(val)))
                }
                Err(err) => Err(format!(
                    "Invalid unicode character(s) in argument {}: {err}",
                    String::from_utf8_lossy(current_arg)
                )
                .into()),
            }
        } else if current_arg.starts_with(b"-") {
            let stripped_arg = &current_arg[1..];
            if stripped_arg.is_empty() {
                return Ok(None);
            }
            let stripped_arg_utf8 = OsStr::from_bytes(stripped_arg).to_string_lossy();

            let offset = self.cursor;
            if let Some(pos) = stripped_arg.iter().position(|x| *x == b'=') {
                if pos == self.cursor + 1 {
                    self.cursor += 1;
                }
            }
            if stripped_arg_utf8.chars().count() > self.cursor + 1 {
                self.cursor += 1;
            } else {
                self.index += 1;
                self.cursor = 0;
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
            self.index += 1;
            Ok(Some(Token::Value(OsStr::from_bytes(current_arg).into())))
        }
    }

    pub fn get_value(&mut self) -> Option<OsString> {
        if self.index >= self.argv.len() {
            return None;
        }

        let current_arg = self.argv[self.index].as_bytes();
        if !current_arg.starts_with(b"-") {
            self.index += 1;
            Some(OsStr::from_bytes(current_arg).into())
        } else if current_arg.starts_with(b"--") && self.cursor > 0 {
            let stripped_arg = &current_arg[2..];
            if stripped_arg.is_empty() {
                return None;
            }
            let offset = self.cursor;
            self.index += 1;
            self.cursor = 0;
            Some(OsStr::from_bytes(&stripped_arg[offset..]).into())
        } else if current_arg.starts_with(b"-") && self.cursor > 0 {
            let stripped_arg = &current_arg[1..];
            if stripped_arg.is_empty() {
                return None;
            }
            let offset = self.cursor;
            self.index += 1;
            self.cursor = 0;
            Some(OsStr::from_bytes(&stripped_arg[offset..]).into())
        } else {
            None
        }
    }

    pub fn finished(&self) -> bool {
        self.index >= self.argv.len()
    }
}

// TODO: Write tests

use std::ffi::OsString;

#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

#[cfg(all(target_os = "wasi", target_env = "p1"))]
use std::{ffi::OsStr, os::wasi::ffi::OsStrExt};

#[cfg(windows)]
use std::os::windows::ffi::{OsStrExt, OsStringExt};

#[derive(Debug)]
pub struct Lexer {
    argv: Vec<OsString>,
    index: usize,
    cursor: usize,
    long_flag: String,
}

#[derive(Debug)]
pub enum Token<'a> {
    ShortFlag(char),
    LongFlag(&'a str),
    Value(OsString),
}

impl Lexer {
    pub fn new(args: impl Iterator<Item = OsString>) -> Self {
        Self {
            argv: args.collect::<Vec<OsString>>(),
            index: 1,
            cursor: 0,
            long_flag: String::new(),
        }
    }

    pub fn starts_with_program_name(mut self, b: bool) -> Self {
        if self.index <= 1 && self.cursor == 0 {
            if b {
                self.index = 1;
            } else {
                self.index = 0;
            }
        }
        self
    }

    #[cfg(not(windows))]
    pub fn next_token(&mut self) -> Result<Option<Token<'_>>, Error> {
        if self.finished() {
            return Ok(None);
        }

        let mut arg = self.argv[self.index].as_bytes();
        if arg.starts_with(b"--") {
            arg = &arg[2..];
            if arg.is_empty() {
                return Ok(None);
            }

            let mut has_value = false;
            if let Some(pos) = arg.iter().position(|x| *x == b'=')
                && pos != 0
            {
                self.cursor = pos + 1;
                arg = &arg[..pos];
                has_value = true;
            }

            match String::from_utf8(arg.into()) {
                Ok(val) => {
                    self.long_flag = val;
                    if !has_value {
                        self.index += 1;
                    }
                    Ok(Some(Token::LongFlag(self.long_flag.as_str())))
                }
                Err(err) => Err(format!(
                    "Invalid unicode character(s) in argument {}: {err}",
                    String::from_utf8_lossy(arg)
                )
                .into()),
            }
        } else if arg.starts_with(b"-") {
            arg = &arg[1..];
            if arg.is_empty() {
                return Ok(None);
            }

            let arg_utf8 = OsStr::from_bytes(arg).to_string_lossy();

            let offset = self.cursor;
            let mut has_value = false;
            if let Some(pos) = arg.iter().position(|x| *x == b'=')
                && pos == self.cursor + 1
            {
                self.cursor = pos + 1;
                has_value = true;
            }

            if !has_value {
                if arg_utf8.chars().count() > self.cursor + 1 {
                    self.cursor += 1;
                } else {
                    self.index += 1;
                    self.cursor = 0;
                }
            }

            if arg_utf8.chars().nth(offset).unwrap() == '�' {
                Err(format!(
                    "Invalid unicode character in {}",
                    String::from_utf8_lossy(arg)
                )
                .into())
            } else {
                Ok(Some(Token::ShortFlag(
                    arg_utf8.chars().nth(offset).unwrap(),
                )))
            }
        } else {
            self.index += 1;
            Ok(Some(Token::Value(OsStr::from_bytes(arg).into())))
        }
    }

    #[cfg(windows)]
    pub fn next_token(&mut self) -> Result<Option<Token<'_>>, Error> {
        if self.finished() {
            return Ok(None);
        }

        let mut arg: Vec<_> = self.argv[self.index].encode_wide().collect();
        const WIDE_DASH: u16 = b'-' as u16;
        if arg.starts_with(&[WIDE_DASH, WIDE_DASH]) {
            arg = arg[2..].to_vec();
            if arg.is_empty() {
                return Ok(None);
            }

            let mut has_value = false;
            if let Some(pos) = arg.iter().position(|x| *x == b'=' as u16)
                && pos != 0
            {
                self.cursor = pos + 1;
                arg = arg[..pos].to_vec();
                has_value = true;
            }

            match String::from_utf16(&arg) {
                Ok(val) => {
                    self.long_flag = val;
                    if !has_value {
                        self.index += 1;
                    }
                    Ok(Some(Token::LongFlag(self.long_flag.as_str())))
                }
                Err(err) => Err(format!(
                    "Invalid unicode character(s) in argument {}: {err}",
                    String::from_utf16_lossy(&arg)
                )
                .into()),
            }
        } else if arg.starts_with(&[WIDE_DASH]) {
            arg = arg[1..].to_vec();
            if arg.is_empty() {
                return Ok(None);
            }

            let arg_utf8 = OsString::from_wide(&arg);
            let arg_utf8 = arg_utf8.to_string_lossy();

            let offset = self.cursor;
            let mut has_value = false;
            if let Some(pos) = arg.iter().position(|x| *x == WIDE_DASH)
                && pos == self.cursor + 1
            {
                self.cursor = pos + 1;
                has_value = true;
            }

            if !has_value {
                if arg_utf8.chars().count() > self.cursor + 1 {
                    self.cursor += 1;
                } else {
                    self.index += 1;
                    self.cursor = 0;
                }
            }

            if arg_utf8.chars().nth(offset).unwrap() == '�' {
                Err(format!(
                    "Invalid unicode character in {}",
                    String::from_utf16_lossy(&arg)
                )
                .into())
            } else {
                Ok(Some(Token::ShortFlag(
                    arg_utf8.chars().nth(offset).unwrap(),
                )))
            }
        } else {
            self.index += 1;
            Ok(Some(Token::Value(OsString::from_wide(&arg))))
        }
    }

    #[cfg(not(windows))]
    pub fn get_value(&mut self) -> Option<OsString> {
        if self.finished() {
            return None;
        }

        let arg = self.argv[self.index].as_bytes();
        if !arg.starts_with(b"-") {
            self.index += 1;
            Some(OsStr::from_bytes(arg).into())
        } else if arg.starts_with(b"--") && self.cursor > 0 {
            let stripped_arg = &arg[2..];
            if stripped_arg.is_empty() {
                return None;
            }
            let offset = self.cursor;
            self.index += 1;
            self.cursor = 0;
            dbg!(OsStr::from_bytes(&stripped_arg[offset..]));
            Some(OsStr::from_bytes(&stripped_arg[offset..]).into())
        } else if arg.starts_with(b"-") && self.cursor > 0 {
            let stripped_arg = &arg[1..];
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

    #[cfg(windows)]
    pub fn get_value(&mut self) -> Option<OsString> {
        if self.finished() {
            return None;
        }

        let arg: Vec<_> = self.argv[self.index].encode_wide().collect();
        const WIDE_DASH: u16 = b'-' as u16;
        if !arg.starts_with(&[WIDE_DASH]) {
            self.index += 1;
            Some(OsString::from_wide(&arg))
        } else if arg.starts_with(&[WIDE_DASH, WIDE_DASH]) && self.cursor > 0 {
            let stripped_arg = &arg[2..];
            if stripped_arg.is_empty() {
                return None;
            }
            let offset = self.cursor;
            self.index += 1;
            self.cursor = 0;
            Some(OsString::from_wide(&stripped_arg[offset..]))
        } else if arg.starts_with(&[WIDE_DASH]) && self.cursor > 0 {
            let stripped_arg = &arg[1..];
            if stripped_arg.is_empty() {
                return None;
            }
            let offset = self.cursor;
            self.index += 1;
            self.cursor = 0;
            Some(OsString::from_wide(&stripped_arg[offset..]))
        } else {
            None
        }
    }

    fn finished(&self) -> bool {
        self.index >= self.argv.len()
    }
}

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::ShortFlag(s) => {
                write!(f, "-{}", *s)
            }
            Token::LongFlag(s) => {
                write!(f, "--{}", *s)
            }
            Token::Value(s) => {
                write!(f, "{}", s.to_string_lossy())
            }
        }
    }
}

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

use std::{
    ffi::{OsStr, OsString},
    str,
};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

pub struct Lexer {
    argv: Vec<OsString>,
    cursor: usize,
    offset: usize,
}

#[derive(Debug)]
pub enum Flag<'a> {
    // Cases:
    //     -a
    //     -a val
    //     -a=val
    //     -aval
    //     -abc
    //     -a val1 val2 val3.. --
    Short(char),
    // Cases:
    //     --key
    //     --key value
    //     --key=value
    //     --key value1 value2 value3.. --
    Long(&'a str),
}

// TODO: Impl `Command` struct that uses builder pattern
// TODO: Impl custom `Error` type

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

    pub fn get_flag(&mut self) -> Option<Flag> {
        if self.cursor >= self.argv.len() {
            return None;
        }

        let current_arg = self.argv[self.cursor].as_bytes();
        if current_arg.starts_with(b"--") {
            let stripped_arg = &current_arg[2..];
            if let Some(pos) = stripped_arg.iter().position(|x| *x == b'=') {
                self.offset = pos + 1;
                match str::from_utf8(&stripped_arg[..pos]) {
                    Ok(val) => Some(Flag::Long(val)),
                    Err(_) => {
                        eprintln!(
                            "Invalid unicode character in \"{}\"",
                            String::from_utf8_lossy(current_arg)
                        );
                        std::process::exit(1);
                    }
                }
            } else {
                self.cursor += 1;
                match str::from_utf8(stripped_arg) {
                    Ok(val) => Some(Flag::Long(val)),
                    Err(_) => {
                        eprintln!(
                            "Invalid unicode character in \"{}\"",
                            String::from_utf8_lossy(current_arg)
                        );
                        std::process::exit(1);
                    }
                }
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

            if stripped_arg_utf8.chars().nth(offset)? == '�' {
                eprintln!(
                    "Invalid unicode character in \"{}\"",
                    String::from_utf8_lossy(current_arg)
                );
                std::process::exit(1);
            } else {
                Some(Flag::Short(stripped_arg_utf8.chars().nth(offset)?))
            }
        } else {
            None
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

// Copyright (C) 2014-2015 Mickaël Salaün
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Lesser General Public License as published by
// the Free Software Foundation, version 3 of the License.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

extern crate collections;

use self::collections::borrow::IntoCow;
use std::error::{Error, FromError};
use std::fmt;
use std::old_io::IoError;

pub struct ParseError {
    desc: String,
    // TODO: cause: Option<&'a (Error + 'a)>,
}

impl ParseError {
    pub fn new(detail: String) -> ParseError {
        ParseError {
            desc: format!("Mount parsing: {}", detail),
        }
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        self.desc.as_slice()
    }
}

impl FromError<IoError> for ParseError {
    fn from_error(err: IoError) -> ParseError {
        ParseError::new(format!("Failed to read the mounts file: {}", err))
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "{}", self.description())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineError<'a> {
    MissingSpec,
    MissingFile,
    InvalidFilePath(&'a str),
    InvalidFile(&'a str),
    MissingVfstype,
    MissingMntops,
    MissingFreq,
    InvalidFreq(&'a str),
    MissingPassno,
    InvalidPassno(&'a str),
}

impl<'a> fmt::Display for LineError<'a> {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let desc = match *self {
            LineError::MissingSpec => "Missing field #1 (spec)".into_cow(),
            LineError::MissingFile => "Missing field #2 (file)".into_cow(),
            LineError::InvalidFilePath(ref f) => format!("Bad field #2 (file) value (not absolute path): {}", f).into_cow(),
            LineError::InvalidFile(ref f) => format!("Bad field #2 (file) value: {}", f).into_cow(),
            LineError::MissingVfstype => "Missing field #3 (vfstype)".into_cow(),
            LineError::MissingMntops => "Missing field #4 (mntops)".into_cow(),
            LineError::MissingFreq => "Missing field #5 (freq)".into_cow(),
            LineError::InvalidFreq(ref f) => format!("Bad field #5 (dump) value: {}", f).into_cow(),
            LineError::MissingPassno => "Missing field #6 (passno)".into_cow(),
            LineError::InvalidPassno(ref f) => format!("Bad field #6 (passno) value: {}", f).into_cow(),
        };
        write!(out, "Line parsing: {}", desc)
    }
}

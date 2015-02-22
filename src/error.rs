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
        ParseError::new(format!("Fail to read the mounts file: {}", err))
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "{}", self.description())
    }
}

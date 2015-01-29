// Copyright (C) 2014 Mickaël Salaün
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

#![feature(collections)]
#![feature(core)]
#![feature(io)]
#![feature(libc)]
#![feature(path)]

extern crate libc;

use libc::c_int;
use std::borrow::Cow::{Borrowed, Owned};
use std::cmp::Ordering;
use std::error::{Error, FromError};
use std::fmt;
use std::io::fs::File;
use std::io::IoError;
use std::str::FromStr;
use std::string::CowString;

const PROC_MOUNTS: &'static str = "/proc/mounts";

pub struct ParseError {
    detail: String,
}

impl ParseError {
    fn new(detail: String) -> ParseError {
        ParseError {
            detail: detail,
        }
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        "Mount parsing"
    }

    fn detail(&self) -> Option<String> {
        Some(self.detail.clone())
    }
}

impl FromError<IoError> for ParseError {
    fn from_error(err: IoError) -> ParseError {
        ParseError::new(format!("Fail to read the mounts file ({})", err))
    }
}

impl fmt::Show for ParseError {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "{}: {}", self.description(), self.detail)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Show)]
pub enum DumpField {
    Ignore = 0,
    Backup = 1,
}

pub type PassField = Option<c_int>;

#[derive(Clone, PartialEq, Eq)]
pub struct Mount {
    pub spec: String,
    pub file: Path,
    pub vfstype: String,
    // TODO: mntops: Vec<MntOps>
    pub mntops: Vec<String>,
    pub freq: DumpField,
    pub passno: PassField,
}

impl Mount {
    pub fn from_str(line: &str) -> Result<Mount, CowString> {
        let line = line.trim();
        let mut tokens = line.split_terminator(|&: s: char| { s == ' ' || s == '\t' })
            .filter(|s| { s != &""  } );
        Ok(Mount {
            spec: try!(tokens.next().ok_or(Borrowed("Missing field #1 (spec)"))).to_string(),
            file: {
                let file = try!(tokens.next().ok_or(Borrowed("Missing field #2 (file)")));
                let path = Path::new_opt(file);
                match path {
                    Some(p) => {
                        if p.is_relative() {
                            return Err(Owned(format!("Bad field #2 (file) value \
                                                     (not absolute path): {}", file)));
                        }
                        p
                    },
                    _ => return Err(Owned(format!("Bad field #2 (file) value: {}", file))),
                }
            },
            vfstype: try!(tokens.next().ok_or(Borrowed("Missing field #3 (vfstype)"))).to_string(),
            mntops: try!(tokens.next().ok_or(Borrowed("Missing field #4 (mntops)")))
                .split_terminator(',').map(|x| { x.to_string() }).collect(),
            freq: {
                let freq = try!(tokens.next().ok_or(Borrowed("Missing field #5 (freq)")));
                match FromStr::from_str(freq) {
                    Some(0) => DumpField::Ignore,
                    Some(1) => DumpField::Backup,
                    _ => return Err(Owned(format!("Bad field #5 (dump) value: {}", freq))),
                }
            },
            passno: {
                let passno = try!(tokens.next().ok_or(Borrowed("Missing field #6 (passno)")));
                match FromStr::from_str(passno) {
                    Some(0) => None,
                    Some(f) if f > 0 => Some(f),
                    _ => return Err(Owned(format!("Bad field #6 (passno) value: {}", passno))),
                }
            },
        })
    }

    // TODO: Return an iterator with `iter_mounts()`
    pub fn get_mounts(root: &Path) -> Result<Vec<Mount>, ParseError> {
        let file = try!(File::open(&Path::new(PROC_MOUNTS)));
        let mut mount = std::io::BufferedReader::new(file);
        let mut ret = vec!();
        for line in mount.lines() {
            let line = try!(line);
            match Mount::from_str(line.as_slice()) {
                Ok(m) => {
                    if root.is_ancestor_of(&m.file) {
                        ret.push(m);
                    }
                },
                Err(e) => return Err(ParseError::new(format!("Fail to parse `{}`: {}", line.trim(), e))),
            }
        }
        Ok(ret)
    }

    pub fn remove_overlaps(mounts: Vec<Mount>) -> Vec<Mount> {
        let mut sorted: Vec<Mount> = vec!();
        let root = Path::new("/");
        'list: for mount in mounts.into_iter().rev() {
            // Strip fake root mounts (created from bind mounts)
            if mount.file == root {
                continue 'list;
            }
            let mut has_overlaps = false;
            'filter: for mount_sorted in sorted.iter() {
                // Check for mount overlaps
                if mount_sorted.file.is_ancestor_of(&mount.file) {
                    has_overlaps = true;
                    break 'filter;
                }
            }
            if !has_overlaps {
                sorted.push(mount);
            }
        }
        sorted.reverse();
        sorted
    }
}

impl fmt::Show for Mount {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "Mount {{ spec: {:?}, file: {:?} vfstype: {:?} mntops: {:?}, freq: {:?}, passno: {:?} }}",
               self.spec, self.file.display(), self.vfstype, self.mntops, self.freq, self.passno)
    }
}

impl FromStr for Mount {
    fn from_str(line: &str) -> Option<Mount> {
        Mount::from_str(line).ok()
    }
}

impl PartialOrd for Mount {
    fn partial_cmp(&self, other: &Mount) -> Option<Ordering> {
        self.file.partial_cmp(&other.file)
    }
}

impl Ord for Mount {
    fn cmp(&self, other: &Mount) -> Ordering {
        self.file.cmp(&other.file)
    }
}

#[test]
fn test_line_root() {
    let root_ref = Mount {
        spec: "rootfs".to_string(),
        file: Path::new("/"),
        vfstype: "rootfs".to_string(),
        mntops: vec!("rw".to_string()),
        freq: DumpField::Ignore,
        passno: None,
    };
    assert_eq!(&Mount::from_str("rootfs / rootfs rw 0 0"), &Ok(root_ref.clone()));
    assert_eq!(&Mount::from_str("rootfs   / rootfs rw 0 0"), &Ok(root_ref.clone()));
    assert_eq!(&Mount::from_str("rootfs	/ rootfs rw 0 0"), &Ok(root_ref.clone()));
    assert_eq!(&Mount::from_str("rootfs / rootfs rw, 0 0"), &Ok(root_ref.clone()));
}

#[test]
fn test_line_mntops() {
    let root_ref = Mount {
        spec: "rootfs".to_string(),
        file: Path::new("/"),
        vfstype: "rootfs".to_string(),
        mntops: vec!("noexec".to_string(), "rw".to_string()),
        freq: DumpField::Ignore,
        passno: None,
    };
    assert_eq!(&Mount::from_str("rootfs / rootfs noexec,rw 0 0"), &Ok(root_ref.clone()));
}

#[cfg(test)]
fn test_file(path: &Path) -> Result<(), String> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Err(format!("Fail to open {}: {}", path.display(), e)),
    };
    let mut mount = std::io::BufferedReader::new(file);
    for line in mount.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => return Err(format!("Fail to read line: {}", e)),
        };
        match Mount::from_str(line.as_slice()) {
            Ok(_) => {},
            Err(e) => return Err(format!("Error for `{}`: {}", line.trim(), e)),
        }
    }
    Ok(())
}

#[test]
fn test_proc_mounts() {
    assert!(test_file(&Path::new("/proc/mounts")).is_ok());
}

#[test]
fn test_path() {
    assert!(Mount::from_str("rootfs ./ rootfs rw 0 0").is_err());
    assert!(Mount::from_str("rootfs foo rootfs rw 0 0").is_err());
    // Should fail for a swap pseudo-mount
    assert!(Mount::from_str("/dev/mapper/swap none swap sw 0 0").is_err());
}

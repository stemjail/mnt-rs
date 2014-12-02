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

extern crate libc;

use libc::c_int;
use std::fmt;
use std::str::{FromStr, MaybeOwned, Owned, Slice};

#[cfg(test)]
use std::io::fs::File;
#[cfg(test)]
use std::io::IoResult;

#[deriving(Clone, PartialEq, Eq, Show)]
pub enum DumpField {
    Ignore = 0,
    Backup = 1,
}

pub type PassField = Option<c_int>;

#[deriving(Clone, PartialEq, Eq)]
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
    pub fn from_str(line: &str) -> Result<Mount, MaybeOwned> {
        let line = line.trim();
        let mut tokens = line.split_terminator(|s: char| { s == ' ' || s == '\t' })
            .filter(|s| { s != &""  } );
        Ok(Mount {
            spec: try!(tokens.next().ok_or(Slice("Missing field #1 (spec)"))).to_string(),
            file: {
                let file = try!(tokens.next().ok_or(Slice("Missing field #2 (file)")));
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
            vfstype: try!(tokens.next().ok_or(Slice("Missing field #3 (vfstype)"))).to_string(),
            mntops: try!(tokens.next().ok_or(Slice("Missing field #4 (mntops)")))
                .split_terminator(',').map(|x| { x.to_string() }).collect(),
            freq: {
                let freq = try!(tokens.next().ok_or(Slice("Missing field #5 (freq)")));
                match from_str::<c_int>(freq) {
                    Some(0) => DumpField::Ignore,
                    Some(1) => DumpField::Backup,
                    _ => return Err(Owned(format!("Bad field #5 (dump) value: {}", freq))),
                }
            },
            passno: {
                let passno = try!(tokens.next().ok_or(Slice("Missing field #6 (passno)")));
                match from_str(passno) {
                    Some(0) => None,
                    Some(f) if f > 0 => Some(f),
                    _ => return Err(Owned(format!("Bad field #6 (passno) value: {}", passno))),
                }
            },
        })
    }
}

impl fmt::Show for Mount {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "Mount {{ spec: {}, file: {} vfstype: {} mntops: {}, freq: {}, passno: {} }}",
               self.spec, self.file.display(), self.vfstype, self.mntops, self.freq, self.passno)
    }
}

impl FromStr for Mount {
    fn from_str(line: &str) -> Option<Mount> {
        Mount::from_str(line).ok()
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

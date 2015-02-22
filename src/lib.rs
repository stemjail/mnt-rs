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

#![feature(collections)]
#![feature(core)]
#![feature(old_io)]
#![feature(libc)]
#![feature(old_path)]

extern crate libc;

use libc::c_int;
use self::error::*;
use std::cmp::Ordering;
use std::fmt;
use std::old_io::fs::File;
use std::str::FromStr;

mod error;

const PROC_MOUNTS: &'static str = "/proc/mounts";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DumpField {
    Ignore = 0,
    Backup = 1,
}

pub type PassField = Option<c_int>;

#[derive(Clone, PartialEq, Eq)]
pub struct MountEntry {
    pub spec: String,
    pub file: Path,
    pub vfstype: String,
    // TODO: mntops: Vec<MntOps>
    pub mntops: Vec<String>,
    pub freq: DumpField,
    pub passno: PassField,
}

impl<'a> FromStr for MountEntry {
    type Err = LineError<'a>;
    fn from_str(line: &str) -> Result<MountEntry, LineError> {
        let line = line.trim();
        let mut tokens = line.split_terminator(|&: s: char| { s == ' ' || s == '\t' })
            .filter(|s| { s != &""  } );
        Ok(MountEntry {
            spec: try!(tokens.next().ok_or(LineError::MissingSpec)).to_string(),
            file: {
                let file = try!(tokens.next().ok_or(LineError::MissingFile));
                let path = Path::new_opt(file);
                match path {
                    Some(p) => {
                        if p.is_relative() {
                            return Err(LineError::InvalidFilePath(file));
                        }
                        p
                    },
                    _ => return Err(LineError::InvalidFile(file)),
                }
            },
            vfstype: try!(tokens.next().ok_or(LineError::MissingVfstype)).to_string(),
            mntops: try!(tokens.next().ok_or(LineError::MissingMntops))
                .split_terminator(',').map(|x| { x.to_string() }).collect(),
            freq: {
                let freq = try!(tokens.next().ok_or(LineError::MissingFreq));
                match FromStr::from_str(freq) {
                    Ok(0) => DumpField::Ignore,
                    Ok(1) => DumpField::Backup,
                    _ => return Err(LineError::InvalidFreq(freq)),
                }
            },
            passno: {
                let passno = try!(tokens.next().ok_or(LineError::MissingPassno));
                match FromStr::from_str(passno) {
                    Ok(0) => None,
                    Ok(f) if f > 0 => Some(f),
                    _ => return Err(LineError::InvalidPassno(passno)),
                }
            },
        })
    }
}

impl MountEntry {
    // TODO: Return an iterator with `iter_mounts()`
    pub fn get_mounts(root: &Path) -> Result<Vec<MountEntry>, ParseError> {
        let file = try!(File::open(&Path::new(PROC_MOUNTS)));
        let mut mount = std::old_io::BufferedReader::new(file);
        let mut ret = vec!();
        let mut line_nb = 0;
        for line in mount.lines() {
            let line = try!(line);
            line_nb += 1;
            match <MountEntry as FromStr>::from_str(line.as_slice()) {
                Ok(m) => {
                    if root.is_ancestor_of(&m.file) {
                        ret.push(m);
                    }
                },
                Err(e) => return Err(ParseError::new(format!("Fail at line {}: {}", line_nb, e))),
            }
        }
        Ok(ret)
    }

    // FIXME: Doesn't work for moved mounts: they don't change order
    pub fn remove_overlaps(mounts: Vec<MountEntry>, exclude_files: &Vec<&Path>) -> Vec<MountEntry> {
        let mut sorted: Vec<MountEntry> = vec!();
        let root = Path::new("/");
        'list: for mount in mounts.into_iter().rev() {
            // Strip fake root mounts (created from bind mounts)
            if mount.file == root {
                continue 'list;
            }
            let mut has_overlaps = false;
            'filter: for mount_sorted in sorted.iter() {
                if exclude_files.iter().skip_while(|&x| mount_sorted.file != **x).next().is_some() {
                    continue 'filter;
                }
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

impl fmt::Debug for MountEntry {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "MountEntry {{ spec: {:?}, file: {:?} vfstype: {:?} mntops: {:?}, freq: {:?}, passno: {:?} }}",
               self.spec, self.file.display(), self.vfstype, self.mntops, self.freq, self.passno)
    }
}

impl PartialOrd for MountEntry {
    fn partial_cmp(&self, other: &MountEntry) -> Option<Ordering> {
        self.file.partial_cmp(&other.file)
    }
}

impl Ord for MountEntry {
    fn cmp(&self, other: &MountEntry) -> Ordering {
        self.file.cmp(&other.file)
    }
}

#[test]
fn test_line_root() {
    let root_ref = MountEntry {
        spec: "rootfs".to_string(),
        file: Path::new("/"),
        vfstype: "rootfs".to_string(),
        mntops: vec!("rw".to_string()),
        freq: DumpField::Ignore,
        passno: None,
    };
    let from_str = <MountEntry as FromStr>::from_str;
    assert_eq!(from_str("rootfs / rootfs rw 0 0"), Ok(root_ref.clone()));
    assert_eq!(from_str("rootfs   / rootfs rw 0 0"), Ok(root_ref.clone()));
    assert_eq!(from_str("rootfs	/ rootfs rw 0 0"), Ok(root_ref.clone()));
    assert_eq!(from_str("rootfs / rootfs rw, 0 0"), Ok(root_ref.clone()));
}

#[test]
fn test_line_mntops() {
    let root_ref = MountEntry {
        spec: "rootfs".to_string(),
        file: Path::new("/"),
        vfstype: "rootfs".to_string(),
        mntops: vec!("noexec".to_string(), "rw".to_string()),
        freq: DumpField::Ignore,
        passno: None,
    };
    let from_str = <MountEntry as FromStr>::from_str;
    assert_eq!(from_str("rootfs / rootfs noexec,rw 0 0"), Ok(root_ref.clone()));
}

#[cfg(test)]
fn test_file(path: &Path) -> Result<(), String> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Err(format!("Fail to open {}: {}", path.display(), e)),
    };
    let mut mount = std::old_io::BufferedReader::new(file);
    for line in mount.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => return Err(format!("Fail to read line: {}", e)),
        };
        match <MountEntry as FromStr>::from_str(line.as_slice()) {
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
    let from_str = <MountEntry as FromStr>::from_str;
    assert!(from_str("rootfs ./ rootfs rw 0 0").is_err());
    assert!(from_str("rootfs foo rootfs rw 0 0").is_err());
    // Should fail for a swap pseudo-mount
    assert!(from_str("/dev/mapper/swap none swap sw 0 0").is_err());
}

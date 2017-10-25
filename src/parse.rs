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

extern crate libc;

use error::*;
use self::libc::c_int;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::convert::{AsRef, From};
use std::fmt;
use std::fs::File;
use std::io::{BufReader, BufRead, Lines};
use std::iter::Enumerate;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const PROC_MOUNTS: &'static str = "/proc/mounts";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DumpField {
    Ignore = 0,
    Backup = 1,
}

pub type PassField = Option<c_int>;

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum MntOps {
    Atime(bool),
    DirAtime(bool),
    RelAtime(bool),
    Dev(bool),
    Exec(bool),
    Suid(bool),
    Write(bool),
    Extra(String),
}

impl FromStr for MntOps {
    type Err = LineError;

    fn from_str(token: &str) -> Result<MntOps, LineError> {
        Ok(match token {
            "atime" => MntOps::Atime(true),
            "noatime" => MntOps::Atime(false),
            "diratime" => MntOps::DirAtime(true),
            "nodiratime" => MntOps::DirAtime(false),
            "relatime" => MntOps::RelAtime(true),
            "norelatime" => MntOps::RelAtime(false),
            "dev" => MntOps::Dev(true),
            "nodev" => MntOps::Dev(false),
            "exec" => MntOps::Exec(true),
            "noexec" => MntOps::Exec(false),
            "suid" => MntOps::Suid(true),
            "nosuid" => MntOps::Suid(false),
            "rw" => MntOps::Write(true),
            "ro" => MntOps::Write(false),
            // TODO: Replace with &str
            extra => MntOps::Extra(extra.to_string()),
        })
    }
}

#[derive(Clone, Debug)]
pub enum Search {
    Spec(String),
    File(PathBuf),
    Vfstype(String),
    Mntopts(Vec<MntOps>),
    Freq(DumpField),
    Passno(PassField),
}

#[derive(Clone, PartialEq, Eq)]
pub struct MountEntry {
    pub spec: String,
    pub file: PathBuf,
    pub vfstype: String,
    pub mntops: Vec<MntOps>,
    pub freq: DumpField,
    pub passno: PassField,
}

impl FromStr for MountEntry {
    type Err = LineError;

    fn from_str(line: &str) -> Result<MountEntry, LineError> {
        let line = line.trim();
        let mut tokens = line.split_terminator(|s: char| { s == ' ' || s == '\t' })
            .filter(|s| { s != &""  } );
        Ok(MountEntry {
            spec: try!(tokens.next().ok_or(LineError::MissingSpec)).to_string(),
            file: {
                let file = try!(tokens.next().ok_or(LineError::MissingFile));
                let path = PathBuf::from(file);
                if path.is_relative() {
                    return Err(LineError::InvalidFilePath(file.into()));
                }
                path
            },
            vfstype: try!(tokens.next().ok_or(LineError::MissingVfstype)).to_string(),
            mntops: try!(tokens.next().ok_or(LineError::MissingMntops))
                // FIXME: Handle MntOps errors
                .split_terminator(',').map(|x| { FromStr::from_str(x).unwrap() }).collect(),
            freq: {
                let freq = try!(tokens.next().ok_or(LineError::MissingFreq));
                match FromStr::from_str(freq) {
                    Ok(0) => DumpField::Ignore,
                    Ok(1) => DumpField::Backup,
                    _ => return Err(LineError::InvalidFreq(freq.into())),
                }
            },
            passno: {
                let passno = try!(tokens.next().ok_or(LineError::MissingPassno));
                match FromStr::from_str(passno) {
                    Ok(0) => None,
                    Ok(f) if f > 0 => Some(f),
                    _ => return Err(LineError::InvalidPassno(passno.into())),
                }
            },
        })
    }
}


/// Get a list of all mount points from `root` and beneath using a custom `BufRead`
pub fn get_submounts_from<T, U>(root: T, iter: MountIter<U>)
        -> Result<Vec<MountEntry>, ParseError> where T: AsRef<Path>, U: BufRead {
    let mut ret = vec!();
    for mount in iter {
        match mount {
            Ok(m) => if m.file.starts_with(&root) {
                ret.push(m);
            },
            Err(e) => return Err(e),
        }
    }
    Ok(ret)
}

/// Get a list of all mount points from `root` and beneath using */proc/mounts*
pub fn get_submounts<T>(root: T) -> Result<Vec<MountEntry>, ParseError> where T: AsRef<Path> {
    get_submounts_from(root, try!(MountIter::new_from_proc()))
}

/// Get the mount point for the `target` using a custom `BufRead`
pub fn get_mount_from<T, U>(target: T, iter: MountIter<U>)
        -> Result<Option<MountEntry>, ParseError> where T: AsRef<Path>, U: BufRead {
    let mut ret = None;
    for mount in iter {
        match mount {
            Ok(m) => if target.as_ref().starts_with(&m.file) {
                // Get the last entry
                ret = Some(m);
            },
            Err(e) => return Err(e),
        }
    }
    Ok(ret)
}

/// Get the mount point for the `target` using */proc/mounts*
pub fn get_mount<T>(target: T) -> Result<Option<MountEntry>, ParseError> where T: AsRef<Path> {
    get_mount_from(target, try!(MountIter::new_from_proc()))
}

/// Get the mount point(s) which match the `search` criteria using a custom `BufRead`
pub fn get_mount_search_from<T>(search: &Search, iter: MountIter<T>)
    -> Result<MountIter<T>, ParseError> where T: BufRead {
    Ok(MountIter::new_search_from_existing(iter, search))
}

/// Get the mount point(s) which match the `search` criteria using */proc/mounts*
pub fn get_mount_search(search: &Search) -> Result<MountIter<BufReader<File>>, ParseError> {
    get_mount_search_from(search, try!(MountIter::new_from_proc()))
}

/// Find the potential mount point providing readable or writable access to a path
///
/// Do not check the path existence but its potentially parent mount point.
pub fn get_mount_writable<T>(target: T, writable: bool) -> Option<MountEntry> where T: AsRef<Path> {
    match get_mount(target) {
        Ok(Some(m)) => {
            if !writable || m.mntops.contains(&MntOps::Write(writable)) {
                Some(m)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub trait VecMountEntry {
    fn remove_overlaps<T>(self, exclude_files: &Vec<T>) -> Self where T: AsRef<Path>;
}

impl VecMountEntry for Vec<MountEntry> {
    // FIXME: Doesn't work for moved mounts: they don't change order
    fn remove_overlaps<T>(self, exclude_files: &Vec<T>) -> Vec<MountEntry> where T: AsRef<Path> {
        let mut sorted: Vec<MountEntry> = vec!();
        let root = Path::new("/");
        'list: for mount in self.into_iter().rev() {
            // Strip fake root mounts (created from bind mounts)
            if AsRef::<Path>::as_ref(&mount.file) == root {
                continue 'list;
            }
            let mut has_overlaps = false;
            'filter: for mount_sorted in sorted.iter() {
                if exclude_files.iter().skip_while(|x|
                       AsRef::<Path>::as_ref(&mount_sorted.file) != x.as_ref()).next().is_some() {
                    continue 'filter;
                }
                // Check for mount overlaps
                if mount.file.starts_with(&mount_sorted.file) {
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
        write!(out, "MountEntry {{ spec: {:?}, file: {:?}, vfstype: {:?}, mntops: {:?}, freq: {:?}, passno: {:?} }}",
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


pub struct MountIter<T> {
    lines: Enumerate<Lines<T>>,
    search: Option<Search>,
}

impl<T> MountIter<T> where T: BufRead {
    pub fn new(mtab: T) -> MountIter<T> {
        MountIter {
            lines: mtab.lines().enumerate(),
            search: None
        }
    }

    pub fn new_search_from_existing(iter: MountIter<T>, search: &Search) -> MountIter<T> {
        MountIter {
            lines: iter.lines,
            search: Some(search.clone())
        }
    }
}

impl MountIter<BufReader<File>> {
    pub fn new_from_proc() -> Result<MountIter<BufReader<File>>, ParseError> {
        let file = try!(File::open(PROC_MOUNTS));
        Ok(MountIter::new(BufReader::new(file)))
    }
}

fn matches(m: &MountEntry, search: &Search) -> bool {
    match *search {
        Search::Spec(ref spec) => {
            if *spec == m.spec {
                return true;
            }
        },
        Search::File(ref file) => {
            if *file == m.file {
                return true;
            }
        },
        Search::Vfstype(ref vfstype) => {
            if *vfstype == m.vfstype {
                return true;
            }
        },
        Search::Mntopts(ref mntops) => {
            // All the opts must be present for a match
            let current_ops: HashSet<_> = m.mntops.iter().cloned().collect();
            let requested_ops: HashSet<_> = mntops.iter().cloned().collect();
            return current_ops.is_superset(&requested_ops);
        },
        Search::Freq(ref dumpfield) => {
            if *dumpfield == m.freq {
                return true;
            }
        },
        Search::Passno(ref passno) => {
            if *passno == m.passno {
                return true;
            }
        }
    }
    false
}


impl<T> Iterator for MountIter<T> where T: BufRead {
    type Item = Result<MountEntry, ParseError>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        loop {
            match self.lines.next() {
                Some((nb, line)) => match line {
                    Ok(line) => match <MountEntry as FromStr>::from_str(line.as_ref()) {
                        Ok(m) => {
                            if let Some(ref s) = self.search {
                                if matches(&m, &s ) {
                                    return Some(Ok(m));
                                } else {
                                    continue;
                                }
                            } else {
                                return Some(Ok(m));
                            }
                        }
                        Err(e) => {
                            return Some(Err(ParseError::new(format!("Failed at line {}: {}", nb, e))));
                        }
                    },
                    Err(e) => {
                        return Some(Err(From::from(e)));
                    },
                },
                None => {
                    return None;
                },
            }
        }
    }
}


#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::{BufReader, BufRead, Cursor};
    use std::path::{Path, PathBuf};
    use std::str::FromStr;
    use super::{DumpField, MountEntry, MountIter, MntOps, get_mount_from, get_submounts_from, get_mount_search_from, Search};

    #[test]
    fn test_line_root() {
        let root_ref = MountEntry {
            spec: "rootfs".to_string(),
            file: PathBuf::from("/"),
            vfstype: "rootfs".to_string(),
            mntops: vec!(MntOps::Write(true)),
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
            file: PathBuf::from("/"),
            vfstype: "rootfs".to_string(),
            mntops: vec!(MntOps::Exec(false), MntOps::Write(true)),
            freq: DumpField::Ignore,
            passno: None,
        };
        let from_str = <MountEntry as FromStr>::from_str;
        assert_eq!(from_str("rootfs / rootfs noexec,rw 0 0"), Ok(root_ref.clone()));
    }

    fn test_file<T>(path: T) -> Result<(), String> where T: AsRef<Path> {
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) => return Err(format!("Failed to open {}: {}", path.as_ref().display(), e)),
        };
        let mount = BufReader::new(file);
        for line in mount.lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => return Err(format!("Failed to read line: {}", e)),
            };
            match <MountEntry as FromStr>::from_str(line.as_ref()) {
                Ok(_) => {},
                Err(e) => return Err(format!("Error for `{}`: {}", line.trim(), e)),
            }
        }
        Ok(())
    }

    #[test]
    fn test_proc_mounts() {
        assert!(test_file("/proc/mounts").is_ok());
    }

    #[test]
    fn test_path() {
        let from_str = <MountEntry as FromStr>::from_str;
        assert!(from_str("rootfs ./ rootfs rw 0 0").is_err());
        assert!(from_str("rootfs foo rootfs rw 0 0").is_err());
        // Should fail for a swap pseudo-mount
        assert!(from_str("/dev/mapper/swap none swap sw 0 0").is_err());
    }

    #[test]
    fn test_proc_mounts_from() {
        use super::MntOps::*;
        use super::DumpField::*;

        let buf = Cursor::new(b"\
            rootfs / rootfs rw 0 0\n\
            sysfs /sys sysfs rw,nosuid,nodev,noexec,relatime 0 0\n\
            tmpfs /sys/fs/cgroup tmpfs ro,nosuid,nodev,noexec,mode=755 0 0\n\
            udev /dev devtmpfs rw,relatime,size=10240k,nr_inodes=505357,mode=755 0 0\n\
            tmpfs /run tmpfs rw,nosuid,relatime,size=809928k,mode=755 0 0\n\
            /dev/mapper/foo-tmp /var/tmp ext4 rw,relatime,data=ordered 0 0\n\
        ".as_ref());
        // FIXME: Append /dev/dm-0 / ext4 rw,relatime,errors=remount-ro,data=ordered 0 0\n\
        let mount_vartmp = MountEntry {
            spec: "/dev/mapper/foo-tmp".to_string(),
            file: PathBuf::from("/var/tmp"),
            vfstype: "ext4".to_string(),
            mntops: vec![Write(true), RelAtime(true), Extra("data=ordered".to_string())],
            freq: Ignore,
            passno: None
        };
        let mount_root = MountEntry {
            spec: "rootfs".to_string(),
            file: PathBuf::from("/"),
            vfstype: "rootfs".to_string(),
            mntops: vec![Write(true)],
            freq: Ignore,
            passno: None
        };
        let mount_sysfs = MountEntry {
            spec: "sysfs".to_string(),
            file: PathBuf::from("/sys"),
            vfstype: "sysfs".to_string(),
            mntops: vec![Write(true), Suid(false), Dev(false), Exec(false), RelAtime(true)],
            freq: Ignore,
            passno: None
        };
        let mount_tmp = MountEntry {
            spec: "tmpfs".to_string(),
            file: PathBuf::from("/sys/fs/cgroup"),
            vfstype: "tmpfs".to_string(),
            mntops: vec![Write(false), Suid(false), Dev(false), Exec(false), Extra("mode=755".to_string())],
            freq: Ignore,
            passno: None
        };
        let mounts_all = vec!(
            mount_root.clone(),
            mount_sysfs.clone(),
            mount_tmp.clone(),
            MountEntry {
                spec: "udev".to_string(),
                file: PathBuf::from("/dev"),
                vfstype: "devtmpfs".to_string(),
                mntops: vec![Write(true), RelAtime(true), Extra("size=10240k".to_string()), Extra("nr_inodes=505357".to_string()), Extra("mode=755".to_string())],
                freq: Ignore,
                passno: None
            },
            MountEntry {
                spec: "tmpfs".to_string(),
                file: PathBuf::from("/run"),
                vfstype: "tmpfs".to_string(),
                mntops: vec![Write(true), Suid(false), RelAtime(true), Extra("size=809928k".to_string()), Extra("mode=755".to_string())],
                freq: Ignore,
                passno: None
            },
            mount_vartmp.clone()
        );
        let mounts = MountIter::new(buf.clone());
        assert_eq!(mounts.map(|x| x.unwrap() ).collect::<Vec<_>>(), mounts_all.clone());
        let mounts = MountIter::new(buf.clone());
        assert_eq!(get_submounts_from("/", mounts).ok(), Some(mounts_all.clone()));
        let mounts = MountIter::new(buf.clone());
        assert_eq!(get_submounts_from("/var/tmp", mounts).ok(), Some(vec!(mount_vartmp.clone())));
        let mounts = MountIter::new(buf.clone());
        assert_eq!(get_mount_from("/var/tmp/bar", mounts).ok(), Some(Some(mount_vartmp.clone())));
        let mounts = MountIter::new(buf.clone());
        assert_eq!(get_mount_from("/var/", mounts).ok(), Some(Some(mount_root.clone())));

        let mounts = MountIter::new(buf.clone());
        assert_eq!(get_mount_search_from(&Search::Spec(String::from("rootfs")), mounts).unwrap().take(1).next().unwrap().unwrap(), mount_root.clone());
        let mounts = MountIter::new(buf.clone());
        assert_eq!(get_mount_search_from(&Search::File(PathBuf::from("/")), mounts).unwrap().take(1).next().unwrap().unwrap(), mount_root.clone());
        let mounts = MountIter::new(buf.clone());
        assert_eq!(get_mount_search_from(&Search::Vfstype(String::from("tmpfs")), mounts).unwrap().take(1).next().unwrap().unwrap(), mount_tmp.clone());
        let mounts = MountIter::new(buf.clone());
        let mnt_ops = vec![MntOps::Write(true), MntOps::Suid(false), MntOps::Dev(false), MntOps::Exec(false)];
        assert_eq!(get_mount_search_from(&Search::Mntopts(mnt_ops), mounts).unwrap().take(1).next().unwrap().unwrap(), mount_sysfs.clone());
        let mounts = MountIter::new(buf.clone());
        assert_eq!(get_mount_search_from(&Search::Freq(DumpField::Ignore), mounts).unwrap().filter_map(Result::ok).collect::<Vec<_>>(), mounts_all.clone());
        let mounts = MountIter::new(buf.clone());
        assert_eq!(get_mount_search_from(&Search::Freq(DumpField::Ignore), mounts).unwrap().filter_map(Result::ok).collect::<Vec<_>>(), mounts_all.clone());
        let mounts = MountIter::new(buf.clone());
        assert_eq!(get_mount_search_from(&Search::Passno(None), mounts).unwrap().filter_map(Result::ok).collect::<Vec<_>>(), mounts_all.clone());
    }
}

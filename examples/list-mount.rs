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

extern crate mnt;

use mnt::get_mount;
use std::env::args;
use std::path::{Path, PathBuf};


fn list_mount(target: &Path) {
    match get_mount(&target) {
        Ok(list) => {
            match list {
                Some(mount) => println!("Mount point: {:?}", mount),
                None => println!("No mount point for {}", target.display()),
            }
        },
        Err(e) => println!("Error: {}", e),
    }
}

fn main() {
    let target = match args().skip(1).next() {
        Some(target) => PathBuf::from(target),
        None => PathBuf::from("/"),
    };
    list_mount(&target);
}

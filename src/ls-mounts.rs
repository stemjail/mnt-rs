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

extern crate mnt;

use mnt::Mount;
use std::io::fs::File;
use std::io::IoResult;

fn list_mounts(path: &Path) -> IoResult<()> {
    let file = try!(File::open(path));
    let mut mount = std::io::BufferedReader::new(file);
    for line in mount.lines() {
        let line = try!(line);
        match Mount::from_str(line.as_slice()) {
            Ok(m) => println!("Found: {}", m),
            Err(e) => println!("Error for `{}`: {}", line.trim(), e),
        }
    }
    Ok(())
}

fn main() {
    let _ = list_mounts(&Path::new("/proc/mounts"));
}

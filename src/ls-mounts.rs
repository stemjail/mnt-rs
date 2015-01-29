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

#![feature(path)]

extern crate mnt;

use mnt::Mount;

fn list_mounts() {
    let root = Path::new("/");
    match Mount::get_mounts(&root) {
        Ok(list) => {
            for mount in Mount::remove_overlaps(list).iter() {
                println!("* {:?}", mount);
            }
        },
        Err(e) => println!("Error: {:?}", e),
    }
}

fn main() {
    list_mounts();
}

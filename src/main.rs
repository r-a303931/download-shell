// download-shell allows downloading files using another IP on the LAN
// Copyright (C) 2025 Andrew Rioux
//
// This program is free software; you can redistribute it and/or
// modify it under the terms of the GNU General Public License
// as published by the Free Software Foundation; either version 2
// of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program; if not, see <https://www.gnu.org/licenses/>.

mod iptc;
mod nl;

fn main() {
    println!("Hi there {} {}", unsafe { libc::geteuid() }, unsafe {
        libc::getuid()
    });
}

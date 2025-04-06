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

fn main() {
    println!(
        "cargo:rustc-link-search=native={}/lib",
        std::env::var("DL_SHELL_LIBNL").unwrap()
    );
    println!("cargo:rustc-link-lib=static=nl-3");
    println!("cargo:rustc-link-lib=static=nl-route-3");

    println!(
        "cargo:rustc-link-search=native={}/lib",
        std::env::var("DL_SHELL_LIBIPTC").unwrap()
    );
    println!("cargo:rustc-link-lib=static=iptc");
    println!("cargo:rustc-link-lib=static=ip4tc");
}

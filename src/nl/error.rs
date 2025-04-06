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

use std::{ffi::CStr, fmt::Display};

use libc::c_int;

use super::ffi::nl_geterror;

#[derive(Debug)]
#[repr(transparent)]
pub struct Error {
    error_code: c_int,
}

impl Error {
    pub(crate) fn new(error_code: c_int) -> Self {
        Error { error_code }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error_msg_utf8 = unsafe {
            let error_msg = nl_geterror(self.error_code);
            let error_msg_ptr = CStr::from_ptr(error_msg);
            std::str::from_utf8(error_msg_ptr.to_bytes()).unwrap()
        };

        write!(f, "internal libnl error: {error_msg_utf8}")
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

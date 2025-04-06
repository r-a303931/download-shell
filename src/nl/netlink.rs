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

use std::{marker::PhantomData, ptr};

use libc::{AF_INET, AF_UNSPEC};

use super::{
    error,
    ffi::*,
    route::{Link, Neigh, Route, RtAddr},
};

/// A netlink socket used to communicate with the kernel
pub struct Socket {
    pub(crate) sock: *mut nl_sock,
}

impl Socket {
    /// Establish a new connection with the Linux kernel
    pub fn new() -> error::Result<Self> {
        unsafe {
            let sock = Socket {
                sock: nl_socket_alloc(),
            };

            let ret = nl_connect(sock.sock, 0);
            if ret < 0 {
                return Err(error::Error::new(ret));
            }

            Ok(sock)
        }
    }

    pub fn get_links(&self) -> error::Result<Cache<Link>> {
        unsafe {
            let mut link_cache = ptr::null_mut::<nl_cache>();

            let ret = rtnl_link_alloc_cache(self.sock, AF_UNSPEC, &mut link_cache as *mut _);

            if ret < 0 {
                return Err(error::Error::new(ret));
            }

            Ok(Cache {
                cache: link_cache,
                dt: PhantomData,
            })
        }
    }

    pub fn get_neigh(&self) -> error::Result<Cache<Neigh>> {
        unsafe {
            let mut neigh_cache = ptr::null_mut::<nl_cache>();

            let ret = rtnl_neigh_alloc_cache(self.sock, &mut neigh_cache as *mut _);

            if ret < 0 {
                return Err(error::Error::new(ret));
            }

            Ok(Cache {
                cache: neigh_cache,
                dt: PhantomData,
            })
        }
    }

    pub fn get_routes(&self) -> error::Result<Cache<Route>> {
        unsafe {
            let mut route_cache = ptr::null_mut::<nl_cache>();

            let ret = rtnl_route_alloc_cache(self.sock, AF_INET, 0, &mut route_cache as *mut _);

            if ret < 0 {
                return Err(error::Error::new(ret));
            }

            Ok(Cache {
                cache: route_cache,
                dt: PhantomData,
            })
        }
    }

    pub fn get_addrs(&self) -> error::Result<Cache<RtAddr>> {
        unsafe {
            let mut addr_cache = ptr::null_mut::<nl_cache>();

            let ret = rtnl_addr_alloc_cache(self.sock, &mut addr_cache as *mut _);

            if ret < 0 {
                return Err(error::Error::new(ret));
            }

            Ok(Cache {
                cache: addr_cache,
                dt: PhantomData,
            })
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe {
            nl_close(self.sock);
        }
    }
}

/// Tries to get a link by the specified ifindex
pub fn get_link_by_index(cache: &Cache<Link>, index: i32) -> Option<Link> {
    unsafe {
        let link = rtnl_link_get(cache.cache, index);

        if link.is_null() {
            return None;
        }

        Some(Link { link })
    }
}

/// Represents the nl_cache in the libnl library, which is itself a general
/// collection of nl_objects
pub struct Cache<T>
where
    T: From<*mut nl_object>,
{
    pub(crate) cache: *mut nl_cache,
    dt: PhantomData<T>,
}

impl<T: From<*mut nl_object>> Cache<T> {
    pub fn iter(&self) -> CacheIter<'_, T> {
        let cache_size = unsafe { nl_cache_nitems(self.cache) } as usize;

        CacheIter {
            obj: unsafe { nl_cache_get_first(self.cache) },
            cache_size,
            index: 0,
            item_type: PhantomData {},
        }
    }
}

impl<T: From<*mut nl_object>> Drop for Cache<T> {
    fn drop(&mut self) {
        unsafe {
            nl_cache_put(self.cache);
        }
    }
}

/// Iterates over caches and provides an easy way to work with them
pub struct CacheIter<'a, T> {
    obj: *mut nl_object,
    cache_size: usize,
    index: usize,
    item_type: PhantomData<&'a T>,
}

impl<T: From<*mut nl_object>> Iterator for CacheIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.index >= self.cache_size {
                return None;
            }

            self.index += 1;

            let obj = self.obj;
            self.obj = unsafe { nl_cache_get_next(obj) };

            if obj.is_null() {
                continue;
            }

            break Some(T::from(obj));
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.cache_size, Some(self.cache_size))
    }
}

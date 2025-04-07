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

use libc::{c_char, c_int, c_uint, c_void};

macro_rules! nl_obj {
    ($name:ident) => {
        #[repr(C)]
        #[allow(non_camel_case_types)]
        pub struct $name {
            _data: [u8; 0],
            _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
        }
    };
}

nl_obj!(nl_sock);
nl_obj!(nl_cache);
nl_obj!(nl_addr);
nl_obj!(nl_object);
nl_obj!(nl_list_head);
nl_obj!(rtnl_addr);
nl_obj!(rtnl_link);
nl_obj!(rtnl_neigh);
nl_obj!(rtnl_route);
nl_obj!(rtnl_nexthop);
nl_obj!(flnl_request);

// from libnl and libnl-route
unsafe extern "C" {
    pub fn nl_socket_alloc() -> *mut nl_sock;
    pub fn nl_socket_free(sock: *mut nl_sock);
    pub fn nl_socket_get_local_port(sock: *const nl_sock) -> u32;
    pub fn nl_connect(sock: *mut nl_sock, protocol: c_int) -> c_int;
    pub fn nl_close(sock: *mut nl_sock) -> c_void;
    pub fn nl_geterror(error: c_int) -> *const c_char;

    pub fn nl_object_put(obj: *mut nl_object) -> c_void;

    pub fn nl_addr_get_len(addr: *mut nl_addr) -> c_uint;
    pub fn nl_addr_get_binary_addr(addr: *mut nl_addr) -> *mut c_void;
    pub fn nl_addr_parse(addrstr: *const i8, hint: c_int, result: *mut *mut nl_addr) -> c_int;
    pub fn nl_addr_put(addr: *mut nl_addr) -> c_void;
    pub fn nl_addr_get_family(addr: *mut nl_addr) -> c_int;
    pub fn nl_addr_get_prefixlen(addr: *mut nl_addr) -> c_uint;

    pub fn nl_cache_foreach(
        cache: *mut nl_cache,
        cb: extern "C" fn(*mut nl_object, *mut c_void),
        arg: *mut c_void,
    ) -> c_void;
    pub fn nl_cache_put(cache: *mut nl_cache) -> c_void;
    pub fn nl_cache_nitems(cache: *mut nl_cache) -> c_int;
    pub fn nl_cache_get_first(cache: *mut nl_cache) -> *mut nl_object;
    pub fn nl_cache_get_next(obj: *mut nl_object) -> *mut nl_object;
    pub fn nl_cache_destroy_and_free(obj: *mut nl_cache) -> c_void;

    pub fn rtnl_addr_alloc_cache(sock: *mut nl_sock, result: *mut *mut nl_cache) -> c_int;
    pub fn rtnl_addr_alloc() -> *mut rtnl_addr;
    pub fn rtnl_addr_get_ifindex(addr: *mut rtnl_addr) -> c_int;
    pub fn rtnl_addr_set_ifindex(addr: *mut rtnl_addr, index: c_int) -> c_int;
    pub fn rtnl_addr_set_prefixlen(addr: *mut rtnl_addr, index: c_int);
    pub fn rtnl_addr_get_family(addr: *mut rtnl_addr) -> c_int;
    pub fn rtnl_addr_get_local(addr: *mut rtnl_addr) -> *mut nl_addr;
    pub fn rtnl_addr_set_local(addr: *mut rtnl_addr, local: *mut nl_addr) -> c_int;
    pub fn rtnl_addr_set_broadcast(addr: *mut rtnl_addr, broadcast: *mut nl_addr) -> c_int;
    pub fn rtnl_addr_add(sock: *mut nl_sock, addr: *mut rtnl_addr, flags: c_int) -> c_int;

    pub fn rtnl_neigh_alloc_cache(sock: *mut nl_sock, result: *mut *mut nl_cache) -> c_int;
    pub fn rtnl_neigh_get(
        cache: *mut nl_cache,
        ifindex: c_int,
        dst: *mut nl_addr,
    ) -> *mut rtnl_neigh;
    pub fn rtnl_neigh_get_dst(neigh: *mut rtnl_neigh) -> *mut nl_addr;
    pub fn rtnl_neigh_get_lladdr(neigh: *mut rtnl_neigh) -> *mut nl_addr;
    pub fn rtnl_neigh_get_ifindex(neigh: *mut rtnl_neigh) -> c_int;

    pub fn rtnl_link_alloc() -> *mut rtnl_link;
    pub fn rtnl_link_veth_alloc() -> *mut rtnl_link;
    pub fn rtnl_link_get(cache: *mut nl_cache, index: c_int) -> *mut rtnl_link;
    pub fn rtnl_link_alloc_cache(
        sock: *mut nl_sock,
        family: c_int,
        result: *mut *mut nl_cache,
    ) -> c_int;
    pub fn rtnl_link_get_addr(link: *mut rtnl_link) -> *mut nl_addr;
    pub fn rtnl_link_get_name(link: *mut rtnl_link) -> *const c_char;
    pub fn rtnl_link_get_ifindex(link: *mut rtnl_link) -> c_int;
    pub fn rtnl_link_get_type(link: *mut rtnl_link) -> *const c_char;
    pub fn rtnl_link_get_flags(link: *mut rtnl_link) -> c_uint;
    pub fn rtnl_link_set_flags(link: *mut rtnl_link, flags: c_uint);
    pub fn rtnl_link_unset_flags(link: *mut rtnl_link, flags: c_uint);
    pub fn rtnl_link_get_mtu(link: *mut rtnl_link) -> c_uint;
    pub fn rtnl_link_set_ns_fd(link: *mut rtnl_link, fd: c_int);
    pub fn rtnl_link_set_name(link: *mut rtnl_link, name: *const c_char);
    pub fn rtnl_link_change(
        sock: *mut nl_sock,
        link: *mut rtnl_link,
        changes: *mut rtnl_link,
        flags: c_int,
    ) -> c_int;
    pub fn rtnl_link_add(sock: *mut nl_sock, link: *const rtnl_link, flags: c_int) -> c_int;
    pub fn rtnl_link_delete(sock: *mut nl_sock, link: *const rtnl_link) -> c_int;
    pub fn rtnl_link_veth_get_peer(link: *mut rtnl_link) -> *mut rtnl_link;

    pub fn rtnl_route_alloc_cache(
        sock: *mut nl_sock,
        family: c_int,
        flags: c_int,
        result: *mut *mut nl_cache,
    ) -> c_int;
    pub fn rtnl_route_get_src(route: *mut rtnl_route) -> *mut nl_addr;
    pub fn rtnl_route_get_dst(route: *mut rtnl_route) -> *mut nl_addr;
    pub fn rtnl_route_get_iif(route: *mut rtnl_route) -> c_int;
    pub fn rtnl_route_get_pref_src(route: *mut rtnl_route) -> *mut nl_addr;
    pub fn rtnl_route_get_nnexthops(route: *mut rtnl_route) -> c_int;
    pub fn rtnl_route_nexthop_n(route: *mut rtnl_route, ind: c_int) -> *mut rtnl_nexthop;

    pub fn rtnl_route_nh_get_gateway(hop: *mut rtnl_nexthop) -> *mut nl_addr;
    pub fn rtnl_route_nh_get_ifindex(hop: *mut rtnl_nexthop) -> c_int;
}

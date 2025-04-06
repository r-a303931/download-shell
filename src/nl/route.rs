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

use std::{
    ffi::{CStr, CString},
    fmt::Debug,
    net::Ipv4Addr,
};

use libc::{c_int, c_uint, AF_INET, AF_LLC};

use super::{
    error,
    netlink::{self, Cache},
};

use super::ffi::*;

/// Represents an address assigned to a link
pub struct RtAddr {
    addr: *mut rtnl_addr,
}

impl RtAddr {
    pub fn local(&self) -> Option<Addr> {
        unsafe {
            let addr = rtnl_addr_get_local(self.addr);

            if addr.is_null() {
                return None;
            }

            Some(Addr { addr })
        }
    }

    pub fn ifindex(&self) -> i32 {
        unsafe { rtnl_addr_get_ifindex(self.addr) }
    }

    pub fn family(&self) -> i32 {
        unsafe { rtnl_addr_get_family(self.addr) }
    }
}

impl From<*mut nl_object> for RtAddr {
    fn from(value: *mut nl_object) -> Self {
        RtAddr {
            addr: value as *mut _,
        }
    }
}

/// Represents a network link, which can represent a network device
pub struct Link {
    pub(crate) link: *mut rtnl_link,
}

impl Link {
    pub const IFF_UP: c_uint = 1 << 0;

    /// Creates a new, empty link object that can be used to issue changes
    pub fn new() -> Self {
        Self {
            link: unsafe { rtnl_link_alloc() },
        }
    }

    /// Create a new empty link that is optimized for virtual ethernet pairing
    pub fn new_veth() -> Self {
        Self {
            link: unsafe { rtnl_link_veth_alloc() },
        }
    }

    /// Apply differences found in the other link object
    pub fn change(&self, socket: &super::netlink::Socket, other: &Link) -> error::Result<()> {
        let ret = unsafe {
            rtnl_link_change(
                socket.sock,
                self.link,
                other.link,
                0x100, /* NLM_F_REPLACE */
            )
        };

        if ret < 0 {
            return Err(error::Error::new(ret));
        }

        Ok(())
    }

    /// Returns the network link name, e.g. eth0
    pub fn name(&self) -> String {
        unsafe {
            let name = rtnl_link_get_name(self.link);
            if name.is_null() {
                return "".to_string();
            }
            let name_rs = CStr::from_ptr(name);
            std::str::from_utf8(name_rs.to_bytes()).unwrap().to_owned()
        }
    }

    /// Provides the address of the link. Can change based on the type of link,
    /// representing MAC addresses or IP addresses
    pub fn addr(&self) -> Addr {
        unsafe {
            Addr {
                addr: rtnl_link_get_addr(self.link),
            }
        }
    }

    /// Returns the MTU of the link
    pub fn mtu(&self) -> u32 {
        unsafe { rtnl_link_get_mtu(self.link) }
    }

    /// Determines the type of link. Ethernet devices are "veth or eth"
    pub fn ltype(&self) -> Option<String> {
        unsafe {
            let ltype = rtnl_link_get_type(self.link);
            if ltype.is_null() {
                return None;
            }
            let ltype_rs = CStr::from_ptr(ltype);
            Some(std::str::from_utf8(ltype_rs.to_bytes()).ok()?.to_owned())
        }
    }

    /// Determines the index of the interface in the kernel table
    pub fn ifindex(&self) -> c_int {
        unsafe { rtnl_link_get_ifindex(self.link) }
    }

    /// Tries to get the neighbor for this link, which can provide the destination address and the
    /// link layer address (lladdr)
    pub fn get_neigh(&self, neigh_table: &Cache<Neigh>, addr: &Addr) -> Option<[u8; 6]> {
        unsafe {
            let neigh = rtnl_neigh_get(neigh_table.cache, self.ifindex(), addr.addr);

            if neigh.is_null() {
                return None;
            }

            Neigh { neigh }.lladdr().hw_address().try_into().ok()
        }
    }

    /// Set the name of an interface
    pub fn set_name(&self, name: &str) {
        unsafe {
            rtnl_link_set_name(self.link, name.as_ptr() as *const _);
        }
    }

    /// Set the namespace file descriptor for an interface
    pub fn set_ns_fd(&self, ns_fd: c_int) {
        unsafe {
            rtnl_link_set_ns_fd(self.link, ns_fd);
        }
    }

    /// Add the link to the running environment
    pub fn add(&self, socket: &super::netlink::Socket, flags: c_int) -> error::Result<()> {
        let ret = unsafe { rtnl_link_add(socket.sock, self.link, flags) };

        if ret < 0 {
            Err(error::Error::new(ret))
        } else {
            Ok(())
        }
    }

    /// Deletes the active link
    pub fn delete(self, socket: &super::netlink::Socket) -> error::Result<()> {
        let ret = unsafe { rtnl_link_delete(socket.sock, self.link) };

        if ret < 0 {
            Err(error::Error::new(ret))
        } else {
            Ok(())
        }
    }

    /// Get the flags on a link
    pub fn get_flags(&self) -> c_uint {
        unsafe { rtnl_link_get_flags(self.link) }
    }

    /// Set flags to ON for a link
    pub fn set_flags(&self, flags: c_uint) {
        unsafe { rtnl_link_set_flags(self.link, flags) }
    }

    /// Toggle flags OFF for a link
    pub fn unset_flags(&self, flags: c_uint) {
        unsafe { rtnl_link_unset_flags(self.link, flags) }
    }

    /// If this is a veth link, return the peer
    pub fn get_peer(&self) -> Option<Self> {
        let link = unsafe { rtnl_link_veth_get_peer(self.link) };

        if link.is_null() {
            return None;
        }

        Some(Self { link })
    }
}

impl Debug for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Link")
            .field("name", &self.name())
            .field("ifindex", &self.ifindex())
            .finish()
    }
}

impl From<*mut nl_object> for Link {
    fn from(value: *mut nl_object) -> Self {
        Self {
            link: value as *mut _,
        }
    }
}

pub fn get_macs_and_src_for_ip(
    addrs: &Cache<RtAddr>,
    routes: &Cache<Route>,
    neighs: &Cache<Neigh>,
    links: &Cache<Link>,
    addr: Ipv4Addr,
) -> Option<(String, i32, Ipv4Addr, [u8; 6], [u8; 6], u8)> {
    let mut sorted_routes = routes.iter().collect::<Vec<_>>();

    sorted_routes.sort_by(|r1, r2| {
        r2.dst()
            .map(|a| a.cidrlen())
            .partial_cmp(&r1.dst().map(|a| a.cidrlen()))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let ip_int = u32::from(addr);

    let route = sorted_routes.iter().find(|route| {
        let Some(dst) = route.dst() else { return false };

        let mask = if dst.cidrlen() != 0 {
            (0xFFFFFFFFu32.overflowing_shr(32 - dst.cidrlen()))
                .0
                .overflowing_shl(32 - dst.cidrlen())
                .0
        } else {
            0
        };

        let Ok(dst_addr): Result<Ipv4Addr, _> = (&dst).try_into() else {
            return false;
        };
        let dst_addr: u32 = dst_addr.into();

        (mask & dst_addr) == (mask & ip_int)
    })?;

    let link_ind = route.hop_iter().next()?.ifindex();

    #[cfg(debug_assertions)]
    {
        println!("Link index: {link_ind}\n");
        for link in links.iter() {
            println!(
                "Link {}: {:?} ({})",
                link.name(),
                link.addr(),
                link.ifindex()
            );

            println!("\tAddrs:");
            for addr in addrs.iter().filter(|addr| addr.ifindex() == link.ifindex()) {
                if let Some(a) = addr.local() {
                    println!("\t\t{:?}", a)
                }
            }

            println!("\tNeighbors:");
            for neigh in neighs
                .iter()
                .filter(|neigh| neigh.ifindex() == link.ifindex())
            {
                println!("\t\t{:?}, {:?}", neigh.dst(), neigh.lladdr());
            }
        }
    }

    let link = netlink::get_link_by_index(links, link_ind)?;

    let neigh = neighs
        .iter()
        .find(|n| n.ifindex() == link.ifindex())
        .map(|n| n.lladdr().hw_address().try_into().ok())
        .flatten()
        .unwrap_or([0xFFu8; 6]);

    let srcip = addrs.iter().find(|a| a.ifindex() == link.ifindex())?;

    Some((
        link.name(),
        link_ind,
        (&srcip.local()?).try_into().ok()?,
        link.addr().hw_address().try_into().ok()?,
        neigh,
        route.dst().unwrap().cidrlen() as u8,
    ))
}

/// Gets the neighbor record for the source IP specified, or get the default address
pub fn get_neigh_for_addr(
    routes: &Cache<Route>,
    neighs: &Cache<Neigh>,
    links: &Cache<Link>,
    addr: &Addr,
) -> Option<(Ipv4Addr, Link, [u8; 6])> {
    for link in links.iter() {
        let Some(neigh) = link.get_neigh(&neighs, addr) else {
            continue;
        };
        return Some((addr.try_into().ok()?, link, neigh));
    }

    // No good neighbors were found above, try to use the default address
    if let Some(def_neigh) = get_default_route(routes) {
        println!("Found default route, trying to get link for it");
        if let Some((laddr, link, neigh)) = neighs
            .iter()
            .filter_map(|n| {
                let Some(link) = netlink::get_link_by_index(links, n.ifindex()) else {
                    return None;
                };

                let Some(first_hop) = def_neigh.hop_iter().next() else {
                    return None;
                };

                if n.ifindex() != first_hop.ifindex() {
                    return None;
                }

                Some(((&first_hop.gateway()?).try_into().ok()?, link, n.lladdr()))
            })
            .next()
        {
            return Some((laddr, link, neigh.hw_address().try_into().ok()?));
        }
    }

    None
}

/// Given the routes cache, returns the default route among them
pub fn get_default_route(routes: &Cache<Route>) -> Option<Route> {
    routes
        .iter()
        .find(|r| r.dst().map(|a| a.cidrlen()).unwrap_or(33) == 0)
}

/// A struct representing the neighbor of a link
pub struct Neigh {
    neigh: *mut rtnl_neigh,
}

impl Neigh {
    /// Pull up the destination address for this neighbor record
    pub fn dst(&self) -> Addr {
        unsafe {
            let addr = rtnl_neigh_get_dst(self.neigh);
            Addr { addr }
        }
    }

    // Bring up the link local address for the neighbor link
    pub fn lladdr(&self) -> Addr {
        unsafe {
            let addr = rtnl_neigh_get_lladdr(self.neigh);
            Addr { addr }
        }
    }

    pub fn ifindex(&self) -> i32 {
        unsafe { rtnl_neigh_get_ifindex(self.neigh) }
    }
}

impl From<*mut nl_object> for Neigh {
    fn from(value: *mut nl_object) -> Self {
        Self {
            neigh: value as *mut _,
        }
    }
}

/// Represents "an address"
/// IPv4? IPv6? MAC? Whatever the "any" or "lo" devices use? Yes!
pub struct Addr {
    addr: *mut nl_addr,
}

impl Addr {
    /// Returns the number of bytes that are in the address
    pub fn len(&self) -> u32 {
        unsafe { nl_addr_get_len(self.addr) }
    }

    /// Returns the address, which can be interpreted based on the results of [`Addr::atype`]
    pub fn hw_address(&self) -> Vec<u8> {
        unsafe {
            let hw_address_ptr = nl_addr_get_binary_addr(self.addr) as *const u8;
            let hw_address_slice = std::slice::from_raw_parts(hw_address_ptr, self.len() as usize);

            hw_address_slice.to_vec()
        }
    }

    // Determines the type of data in [`Addr::hw_address`]
    pub fn atype(&self) -> Option<c_int> {
        if self.addr.is_null() {
            None
        } else {
            Some(unsafe { nl_addr_get_family(self.addr) })
        }
    }

    /// Returns the length of the subnet mask applying to this address
    pub fn cidrlen(&self) -> c_uint {
        unsafe { nl_addr_get_prefixlen(self.addr) }
    }
}

impl Debug for Addr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = match self.atype() {
            Some(AF_INET) => {
                let octets = self.hw_address();
                f.debug_struct("Addr")
                    .field(
                        "addr",
                        &format!(
                            "{}.{}.{}.{}/{}",
                            octets[0],
                            octets[1],
                            octets[2],
                            octets[3],
                            self.cidrlen()
                        ),
                    )
                    .finish()
            }
            Some(AF_LLC) => {
                let octets = self.hw_address();

                f.debug_struct("Addr")
                    .field(
                        "addr",
                        &format!(
                            "{:02X?}:{:02X?}:{:02X?}:{:02X?}:{:02X?}:{:02X?}",
                            octets[0], octets[1], octets[2], octets[3], octets[4], octets[5],
                        ),
                    )
                    .finish()
            }
            None => f
                .debug_struct("Addr")
                .field("addr", &"unknown")
                .field("atype", &"unknown")
                .finish(),
            _ => f
                .debug_struct("Addr")
                .field("addr", &self.hw_address())
                .field("atype", &self.atype())
                .finish(),
        };
        res
    }
}

impl From<Ipv4Addr> for Addr {
    fn from(value: Ipv4Addr) -> Self {
        unsafe {
            let mut addr = std::ptr::null_mut::<nl_addr>();
            let value = CString::new(format!("{value}")).unwrap();

            // we can ignore the return code because it is guaranteed to not be invalid
            nl_addr_parse(value.as_ptr(), AF_INET, &mut addr as *mut _);

            Addr { addr }
        }
    }
}

impl TryFrom<&Addr> for Ipv4Addr {
    type Error = error::Error;

    fn try_from(value: &Addr) -> Result<Self, Self::Error> {
        if value.len() != 4 {
            return Err(error::Error::new(15 /* NL_AF_MISMATCH */));
        }

        let addr = value.hw_address();
        Ok(Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]))
    }
}

/// Represents a route in the kernel routing table
pub struct Route {
    route: *mut rtnl_route,
}

impl Route {
    /// Represents the destination of the route
    pub fn src(&self) -> Option<Addr> {
        unsafe {
            let addr = rtnl_route_get_src(self.route);

            if addr.is_null() {
                return None;
            }

            Some(Addr { addr })
        }
    }

    /// Represents the destination of the route
    pub fn dst(&self) -> Option<Addr> {
        unsafe {
            let addr = rtnl_route_get_dst(self.route);

            if addr.is_null() {
                return None;
            }

            Some(Addr { addr })
        }
    }

    /// Returns the amount of hops are in this route
    pub fn nexthop_len(&self) -> c_int {
        unsafe { rtnl_route_get_nnexthops(self.route) }
    }

    /// Gets the hop at the index specify
    pub fn nexthop(&self, ind: i32) -> Option<Nexthop> {
        unsafe {
            let nexthop = rtnl_route_nexthop_n(self.route, ind);
            if nexthop.is_null() {
                return None;
            }
            Some(Nexthop { nexthop })
        }
    }

    /// Returns an iterator representing all the hops for this route
    pub fn hop_iter(&self) -> NexthopIter<'_> {
        NexthopIter {
            route: &self,
            index: 0,
        }
    }
}

impl From<*mut nl_object> for Route {
    fn from(value: *mut nl_object) -> Self {
        Route {
            route: value as *mut _,
        }
    }
}

/// Represents the hops of a network route
pub struct Nexthop {
    nexthop: *mut rtnl_nexthop,
}

impl Nexthop {
    /// Returns the gateway used for this network hop
    pub fn gateway(&self) -> Option<Addr> {
        unsafe {
            let addr = rtnl_route_nh_get_gateway(self.nexthop);

            if addr.is_null() {
                return None;
            }

            Some(Addr { addr })
        }
    }

    /// Returns the interface index for this network hop
    pub fn ifindex(&self) -> i32 {
        unsafe { rtnl_route_nh_get_ifindex(self.nexthop) }
    }
}

/// An iterator for working with route hops
pub struct NexthopIter<'a> {
    route: &'a Route,
    index: i32,
}

impl Iterator for NexthopIter<'_> {
    type Item = Nexthop;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.route.nexthop(self.index);

        if next.is_none() {
            return None;
        }

        self.index += 1;

        next
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.route.nexthop_len() as usize,
            Some(self.route.nexthop_len() as usize),
        )
    }
}

/// Determines the source IP address to use in order to make a network request
pub fn get_srcip_for_dstip(routes: &Cache<Route>, ip: Ipv4Addr) -> Option<Ipv4Addr> {
    let mut sorted_routes = routes.iter().collect::<Vec<_>>();

    sorted_routes.sort_by(|r1, r2| {
        r2.dst()
            .map(|a| a.cidrlen())
            .partial_cmp(&r1.dst().map(|a| a.cidrlen()))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let ip_int = u32::from(ip);

    sorted_routes
        .iter()
        .filter(|route| {
            let Some(dst) = route.dst() else { return false };

            let mask = if dst.cidrlen() != 0 {
                (0xFFFFFFFFu32.overflowing_shr(32 - dst.cidrlen()))
                    .0
                    .overflowing_shl(32 - dst.cidrlen())
                    .0
            } else {
                0
            };

            let Ok(dst_addr): Result<Ipv4Addr, _> = (&dst).try_into() else {
                return false;
            };
            let dst_addr: u32 = dst_addr.into();

            (mask & dst_addr) == (mask & ip_int)
        })
        .filter_map(|route| {
            route
                .hop_iter()
                .next()
                .and_then(|hop| hop.gateway())
                .or(route.dst())
        })
        .filter_map(|gateway| (&gateway).try_into().ok())
        .next()
}

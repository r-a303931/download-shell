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

use std::net::Ipv4Addr;

use anyhow::Context;

mod nl;

#[derive(Debug)]
struct Args {
    program: String,
    program_args: Vec<String>,
    source_ip: Option<Ipv4Addr>,
}

fn parse_args() -> Args {
    let mut program = "/bin/sh".to_owned();
    let mut source_ip = None::<Ipv4Addr>;

    let mut args = std::env::args();
    args.next();
    while let Some(arg) = args.next().take() {
        match &*arg {
            "-s" | "--source-ip" => match args.next().take().map(|s| s.parse()) {
                Some(Ok(ip)) => source_ip = Some(ip),
                Some(Err(e)) => {
                    eprintln!("Error parsing source IP address: {e}");
                }
                None => {
                    eprintln!("Error: source IP address not provided");
                }
            },
            _ => {
                program = arg;
                break;
            }
        }
    }

    let mut program_args = args.collect::<Vec<_>>();
    program_args.insert(0, program.clone());

    Args {
        program,
        program_args,
        source_ip,
    }
}

/// Find an available IP range that can be used to tunnel traffic
/// between the new namespace and the host system
fn find_tunnel_ip_range(routes: &nl::netlink::Cache<nl::route::Route>) -> anyhow::Result<Ipv4Addr> {
    let mut result_ip = Ipv4Addr::new(172, 16, 0, 0);

    let mut routes = routes.iter().collect::<Vec<_>>();

    routes.sort_by(|r1, r2| {
        r1.dst()
            .and_then(|a| {
                let a: Option<Ipv4Addr> = (&a).try_into().ok();
                a.map(|ip| -> u32 { ip.into() })
            })
            .partial_cmp(
                &r2.dst()
                    .and_then(|a| (&a).try_into().ok().map(|ip: Ipv4Addr| ip.into())),
            )
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for route in routes {
        let Some(dst) = route.dst() else {
            continue;
        };

        if dst.cidrlen() == 0 {
            continue;
        }

        let Ok(dst_addr): Result<Ipv4Addr, _> = (&dst).try_into() else {
            continue;
        };
        let dst_addr: u32 = dst_addr.into();

        if dst_addr & 0xFFF00000 != 0xAC100000 {
            continue;
        }

        let mask = (0xFFFFFFFFu32.overflowing_shr(32 - dst.cidrlen()))
            .0
            .overflowing_shl(32 - dst.cidrlen())
            .0;

        let res_ip_u32: u32 = result_ip.into();
        if (dst_addr & mask) == (res_ip_u32 & mask) {
            let next_net = 0xFFFFFFFFu32.overflowing_shr(dst.cidrlen()).0 + 1;
            let res_ip_u32 = dst_addr + next_net;
            result_ip = res_ip_u32.into();
        }
    }

    let res_ip_u32: u32 = result_ip.into();
    if res_ip_u32 & 0xFFF00000 != 0xAC100000 {
        anyhow::bail!("Unable to find a tunnel IP address in the 172.16.0.0/16 range!");
    }

    Ok(result_ip)
}

fn main() -> anyhow::Result<()> {
    // This Rust program is based on a bash script, found in the root
    // of this git repo called download-shell.sh

    // The reason it is written is because sometimes systems don't have
    // a new enough version of the `ip` utility to create namespaces,
    // even though the Linux kernel supports it as far back as in
    // version 2.4

    // To ease the transition, most blocks of code will be marked
    // with a line number and bash command, referencing download-shell.sh

    // While most of the commands are the same, the way that the `ip` utility
    // handles network namespaces is optimized for CLI usage. This program
    // instead chooses to use anonymous namespaces created via `unshare`, and
    // so most of the bash commands will still map with the exception of the
    // namespace create and delete commands. However, they will appear
    // in a different order

    // 3-6: Root check
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("This program needs to be run as root");
        std::process::exit(1);
    }

    let args = parse_args();

    // 13: Debug statement
    match &args.source_ip {
        Some(ip) => println!("Sending traffic out as {ip:?}..."),
        None => println!("Sending traffic using the host IP address"),
    }

    let nl_sock = nl::netlink::Socket::new().context("Could not allocate Netlink socket")?;
    let routes = nl_sock
        .get_routes()
        .context("Could not initially load routes")?;

    let tunnel_net_id: u32 = find_tunnel_ip_range(&routes)?.into();

    let host_link_name = format!("dlsh{}.0", unsafe { libc::getpid() });
    let container_link_name = format!("dlsh{}.1", unsafe { libc::getpid() });

    // 15: ip link add downloader.0 type veth peer name downloader.1
    let (links, host_link, container_link) = {
        let link = nl::route::Link::new_veth();
        let peer = link.get_peer().ok_or(anyhow::anyhow!(
            "Could not get peer link for download tunnel"
        ))?;

        link.set_name(&host_link_name);
        peer.set_name(&container_link_name);

        link.add(&nl_sock, 0x200 | 0x400 /* NLM_F_CREATE | NLM_F_EXCL */)?;

        let links = nl_sock
            .get_links()
            .context("Could not acquire link list for adding veth device")?;

        let link = links
            .iter()
            .find(|l| l.name() == host_link_name)
            .ok_or(anyhow::anyhow!(
                "Could not get host link for download tunnel"
            ))?;
        let peer = links
            .iter()
            .find(|l| l.name() == container_link_name)
            .ok_or(anyhow::anyhow!(
                "Could not get peer link for download tunnel"
            ))?;

        (links, link, peer)
    };

    // 16: ip netns add downloader
    {
        // Block left empty, to acknowledge the line of bash that
        // doesn't get to be reimplemented
    }

    // 17: ip link set downloader.0 up
    {
        let up = nl::route::Link::new();
        up.set_flags(nl::route::Link::IFF_UP);
        host_link
            .change(&nl_sock, &up)
            .context("Could not set downloader interface to be up")?;
    }

    let host_tunnel_ip: Ipv4Addr = (tunnel_net_id + 1).into();
    let container_tunnel_ip: Ipv4Addr = (tunnel_net_id + 2).into();
    let tunnel_broadcast_ip: Ipv4Addr = (tunnel_net_id + 3).into();
    // 20: ip addr add 172.31.254.253/30 dev downloader.0
    {
        let local_ip = nl::route::Addr::from(host_tunnel_ip);
        let broadcast_ip = nl::route::Addr::from(tunnel_broadcast_ip);
        let rt_local_ip = nl::route::RtAddr::new()
            .ok_or(anyhow::anyhow!("Could not allocate new tunnel IP address"))?;

        rt_local_ip
            .set_local(local_ip)
            .context("Could not set the address of the host interface")?;
        rt_local_ip.set_ifindex(host_link.ifindex());
        rt_local_ip
            .set_broadcast(broadcast_ip)
            .context("Could not set the broadcast IP of the host interface")?;
        rt_local_ip.set_prefixlen(30);

        rt_local_ip
            .add(&nl_sock, 0x200)
            .context("Could not add the IP address to the host tunnel interface")?;
    }

    // Lines 18 and 22-25 need to be done after forking and unshare

    // 27: DEFAULT_IF="$(ip r | grep default | sed -nE 's/^.*dev ([^ ]*) ?.*/\1/p')""
    let default_if = {
        let default_route = routes
            .iter()
            .find(|r| r.dst().map(|a| a.cidrlen() == 0).unwrap_or(false))
            .ok_or(anyhow::anyhow!("Could not find the default route"))?;

        let local_hop = default_route
            .hop_iter()
            .next()
            .ok_or(anyhow::anyhow!(
                "Could not get the local interface for the default route gateway"
            ))?
            .ifindex();

        links
            .iter()
            .find(|l| l.ifindex() == local_hop)
            .ok_or(anyhow::anyhow!(
                "Could not find the interface associated with the default route"
            ))?
    };

    // 29: echo 1 > /proc/sys/net/ipv4/ip_forward
    std::fs::write("/proc/sys/net/ipv4/ip_forward", b"1")
        .context("could not enable IP forwarding")?;

    // Having a consistent comment makes the cleanup that comes later a lot easier
    let firewall_comment = format!("dlsh{}", unsafe { libc::getpid() });

    // 31: If a source IP is specified
    match &args.source_ip {
        None => {
            // 32: iptables -t nat -A POSTROUTING -o "$DEFAULT_IF" -j MASQUERADE
            std::process::Command::new("iptables")
                .args([
                    "-t",
                    "nat",
                    "-A",
                    "POSTROUTING",
                    "-o",
                    &default_if.name(),
                    "-j",
                    "MASQUERADE",
                    "-m",
                    "comment",
                    "--comment",
                    &firewall_comment,
                ])
                .output()
                .context("Could not create the MASQUERADE rule")?;
        }
        Some(ip) => {
            // 34: iptables -t nat -A POSTROUTING -s 172.31.254.254 -j SNAT --to-source $1
            std::process::Command::new("iptables")
                .args([
                    "-t",
                    "nat",
                    "-A",
                    "POSTROUTING",
                    "-s",
                    &format!("{container_tunnel_ip}"),
                    "-j",
                    "SNAT",
                    "--to-source",
                    &format!("{ip}"),
                    "-m",
                    "comment",
                    "--comment",
                    &firewall_comment,
                ])
                .output()
                .context("Could not create source NAT rule")?;

            // 36: echo 1 > /proc/sys/net/ipv4/conf/all/proxy_arp
            std::fs::write("/proc/sys/net/ipv4/conf/all/proxy_arp", b"1")
                .context("could not enable proxy_arp")?;
            // 37: echo 1 > /proc/sys/net/ipv4/conf/$DEFAULT_IF/proxy_arp
            std::fs::write(
                &format!("/proc/sys/net/ipv4/conf/{}/proxy_arp", &default_if.name()),
                b"1",
            )
            .context("could not enable proxy arp for interface")?;

            // 38: ip route add $1/32 dev downloader.0
            {
                let hop = nl::route::Nexthop::new()
                    .ok_or(anyhow::anyhow!("Could not allocate a new nexthop object"))?;

                hop.set_ifindex(host_link.ifindex());

                let new_route = nl::route::Route::new().ok_or(anyhow::anyhow!(
                    "Could not allocate a new route object for ARP proxy"
                ))?;

                let target_addr = nl::route::Addr::from(*ip);
                target_addr.set_cidrlen(32);

                new_route.add_nexthop(&hop);
                new_route.set_dst(target_addr);

                new_route.add(&nl_sock, 0x400)?;
            }
        }
    }

    // iptables -t filter -A FORWARD -s 172.31.254.254 -j ACCEPT
    std::process::Command::new("iptables")
        .args([
            "-t",
            "filter",
            "-A",
            "FORWARD",
            "-s",
            &format!("{container_tunnel_ip}"),
            "-j",
            "ACCEPT",
            "-m",
            "comment",
            "--comment",
            &firewall_comment,
        ])
        .output()
        .context("could not add firewall rule to allow traffic forwarding")?;

    let (unshare_semaphore, movelink_semaphore) = unsafe {
        let unshare_semaphore = libc::mmap(
            std::ptr::null_mut(),
            std::mem::size_of::<libc::sem_t>(),
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_ANONYMOUS | libc::MAP_SHARED,
            0,
            0,
        ) as *mut libc::sem_t;
        let ret = libc::sem_init(unshare_semaphore, 1, 0);
        if ret != 0 {
            Err(std::io::Error::from_raw_os_error(ret))
                .context("could not initialize the semaphore for unshare")?;
        }

        let movelink_semaphore = libc::mmap(
            std::ptr::null_mut(),
            std::mem::size_of::<libc::sem_t>(),
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_ANONYMOUS | libc::MAP_SHARED,
            0,
            0,
        ) as *mut libc::sem_t;
        let ret = libc::sem_init(movelink_semaphore, 1, 1);
        if ret != 0 {
            Err(std::io::Error::from_raw_os_error(ret))
                .context("could not initialize the semaphore for moving links")?;
        }

        (unshare_semaphore, movelink_semaphore)
    };

    let child = unsafe { libc::fork() };

    match child {
        // Error
        ..0 => {
            // Error out
            println!("Error forking!");
            std::process::exit(3);
        }
        // Child
        0 => {
            drop(nl_sock);

            // 16: ip netns add downloader
            {
                let unshare_result =
                    unsafe { libc::unshare(libc::CLONE_NEWNS | libc::CLONE_NEWNET) };

                if unshare_result < 0 {
                    eprintln!("Failed to unshare! {:?}", std::io::Error::last_os_error());
                    std::process::exit(2);
                }

                unsafe {
                    let ret = libc::sem_post(unshare_semaphore);
                    if ret != 0 {
                        Err(std::io::Error::from_raw_os_error(ret))
                            .context("child: could not signal unshare complete")?;
                    }
                }
            }

            // 18: ip link set downloader.1 netns downloader
            unsafe {
                let ret = libc::sem_wait(movelink_semaphore);
                if ret != 0 {
                    Err(std::io::Error::from_raw_os_error(ret))
                        .context("child: could not wait for link to be moved to namespace")?;
                }
            }

            let nl_sock =
                nl::netlink::Socket::new().context("child: could not get new netlink socket")?;
            let links = nl_sock
                .get_links()
                .context("child: could not get new links object")?;

            let set_interface_up = nl::route::Link::new();
            set_interface_up.set_flags(nl::route::Link::IFF_UP);

            // 22: ip -n downloader link set lo up
            {
                let lo = links
                    .iter()
                    .find(|l| l.name() == "lo")
                    .ok_or(anyhow::anyhow!("Could not find lo loopback interface!"))?;
                lo.change(&nl_sock, &set_interface_up)
                    .context("child: could not set loopback up")?;
            }

            // 23: ip -n downloader link set downloader.1 up
            container_link
                .change(&nl_sock, &set_interface_up)
                .context("child: could not set container interface up")?;

            // 24: ip -n downloader addr add 172.31.254.254/30 dev downloader.1
            {
                let local_ip = nl::route::Addr::from(container_tunnel_ip);
                let broadcast_ip = nl::route::Addr::from(tunnel_broadcast_ip);
                let rt_local_ip = nl::route::RtAddr::new()
                    .ok_or(anyhow::anyhow!("Could not allocate new tunnel IP address"))?;

                rt_local_ip
                    .set_local(local_ip)
                    .context("child: could not set host IP for tunnel route")?;
                rt_local_ip.set_ifindex(container_link.ifindex());
                rt_local_ip
                    .set_broadcast(broadcast_ip)
                    .context("child: could not set broadcast for tunnel route")?;
                rt_local_ip.set_prefixlen(30);

                rt_local_ip
                    .add(&nl_sock, 0x200)
                    .context("child: could not create tunnel route")?;
            }

            // 25: ip -n downloader route add default via 172.31.254.253
            {
                let hop = nl::route::Nexthop::new()
                    .ok_or(anyhow::anyhow!("Could not allocate a new nexthop object"))?;

                let gateway = nl::route::Addr::from(host_tunnel_ip);

                hop.set_ifindex(container_link.ifindex());
                hop.set_gateway(gateway);

                let new_route = nl::route::Route::new().ok_or(anyhow::anyhow!(
                    "Could not allocate a new default route object for the namespace"
                ))?;

                let default_route = nl::route::Addr::from(Ipv4Addr::new(0, 0, 0, 0));
                default_route.set_cidrlen(0);

                new_route.add_nexthop(&hop);
                new_route.set_dst(default_route);

                new_route
                    .add(&nl_sock, 0x400)
                    .context("child: could not create default route")?;
            }

            // 41: ip netns exec downloader bash
            {
                // TODO: remount /sys

                let argv: Vec<*const std::ffi::c_char> = args
                    .program_args
                    .iter()
                    .map(|s| s.as_ptr() as *const i8)
                    .chain(Some(std::ptr::null()))
                    .collect();

                let env: Vec<String> = std::env::vars()
                    .map(|(k, v)| {
                        if k == "PS1" {
                            format!("PS1=(download-shell) {v}")
                        } else {
                            format!("{k}={v}")
                        }
                    })
                    .collect();

                let envp: Vec<*const std::ffi::c_char> = env
                    .iter()
                    .map(|m| m.as_ptr() as *const _)
                    .chain(Some(std::ptr::null()))
                    .collect();

                let program = args.program.clone();

                unsafe {
                    libc::execve(program.as_ptr() as *const i8, argv.as_ptr(), envp.as_ptr())
                };

                Err(std::io::Error::last_os_error())?;
            }
        }
        // Parent
        1.. => {
            // 16: ip netns add downloader
            unsafe {
                let ret = libc::sem_wait(unshare_semaphore);
                if ret != 0 {
                    Err(std::io::Error::from_raw_os_error(ret))
                        .context("parent: could not wait for unshare")?;
                }
            };

            // 18: ip link set downloader.1 netns downloader
            {
                let changes = nl::route::Link::new();
                changes.set_ns_pid(child);
                container_link
                    .change(&nl_sock, &changes)
                    .context("parent: could not move device to namespace")?;

                unsafe {
                    let ret = libc::sem_post(movelink_semaphore);
                    if ret != 0 {
                        Err(std::io::Error::from_raw_os_error(ret))
                            .context("parent: could not signal device move")?;
                    }
                }
            }

            // 41: ip netns exec downloader bash
            {
                let mut status = 0;
                unsafe {
                    libc::waitpid(child, &mut status, 0);
                    libc::kill(child, libc::SIGKILL);
                }
            }

            // 43: ip netns delete downloader
            // Implicitly performed by the child process dying
        }
    }

    // Find the firewall rules with the comment specified above and delete them
    let clean_iptables = |table: &str, chain: &str| -> anyhow::Result<()> {
        let current_rules = std::process::Command::new("iptables")
            .args(["-t", table, "--line-numbers", "-vn", "-L", chain])
            .output()
            .context("could not list firewall rules")?
            .stdout;

        let output_utf8 = std::str::from_utf8(&current_rules)?;

        let Some(rule_line) = output_utf8
            .lines()
            .find(|l| l.contains(&format!("/* {firewall_comment} */")))
        else {
            eprintln!("warning: could not clear out firewall rules from the {table} table: could not find rule");
            return Ok(());
        };

        let rule_num: u16 = rule_line
            .split_ascii_whitespace()
            .next()
            .ok_or(anyhow::anyhow!("warning: could not clear out firewall rules from the {table} table: could not parse rule number"))?
            .parse()?;

        std::process::Command::new("iptables")
            .args(["-t", table, "-D", chain, &format!("{rule_num}")])
            .output()
            .context("could not delete firewall rule")?;

        Ok(())
    };

    clean_iptables("filter", "FORWARD").context("could not clear filter rule")?;
    clean_iptables("nat", "POSTROUTING").context("could not clear NAT rule")?;

    Ok(())
}

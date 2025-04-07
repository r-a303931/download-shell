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

mod iptc;
mod nl;

struct Args {
    program: String,
    program_args: Vec<String>,
    source_ip: Option<Ipv4Addr>,
}

fn parse_args() -> Args {
    let mut program = "/bin/sh".to_owned();
    let mut source_ip = None::<Ipv4Addr>;

    let mut args = std::env::args();
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
                break;
            }
        }
    }

    if let Some(prog) = args.next() {
        program = prog;
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

    let nl_sock = nl::netlink::Socket::new()?;
    let routes = nl_sock.get_routes()?;

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

        let links = nl_sock.get_links()?;

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
        host_link.change(&nl_sock, &up)?;
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

        rt_local_ip.set_local(local_ip)?;
        rt_local_ip.set_ifindex(host_link.ifindex());
        rt_local_ip.set_broadcast(broadcast_ip)?;
        rt_local_ip.set_prefixlen(30);

        rt_local_ip.add(nl_sock, 0x200)?;
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
    std::fs::write("/proc/sys/net/ipv4/ip_forward", "1")?;

    // 31: If a source IP is specified
    match &args.source_ip {
        None => {
            // 32: iptables -t nat -A POSTROUTING -o "$DEFAULT_IF" -j MASQUERADE
        }
        Some(ip) => {
            // 34: iptables -t nat -A POSTROUTING -s 172.31.254.254 -j SNAT --to-source $1
            {}

            // 36: echo 1 > /proc/sys/net/ipv4/conf/all/proxy_arp
            std::fs::write("/proc/sys/net/ipv4/conf/all/proxy_arp", "1")?;
            // 37: echo 1 > /proc/sys/net/ipv4/conf/$DEFAULT_IF/proxy_arp
            std::fs::write(
                &format!("/proc/sys/net/ipv4/conf/{}/proxy_arp", &default_if.name()),
                "1",
            )?;

            // 38: ip route add $1/32 dev downloader.0
            {}
        }
    }

    // iptables -t filter -A FORWARD -s 172.31.254.254 -j ACCEPT
    {}

    let (unshare_semaphore, movelink_semaphore) = unsafe {
        let unshare_semaphore = libc::mmap(
            std::ptr::null_mut(),
            std::mem::size_of::<libc::sem_t>(),
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_ANONYMOUS | libc::MAP_SHARED,
            0,
            0,
        ) as *mut libc::sem_t;
        libc::sem_init(unshare_semaphore, 1, 0);

        let movelink_semaphore = libc::mmap(
            std::ptr::null_mut(),
            std::mem::size_of::<libc::sem_t>(),
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_ANONYMOUS | libc::MAP_SHARED,
            0,
            0,
        ) as *mut libc::sem_t;
        libc::sem_init(movelink_semaphore, 1, 1);

        (unshare_semaphore, movelink_semaphore)
    };

    let child = unsafe { libc::fork() };

    match child {
        // Error
        ..0 => {
            // Error out
        }
        // Child
        0 => {
            // 16: ip netns add downloader
            println!("child: Unsharing!");
            {
                let unshare_result =
                    unsafe { libc::unshare(libc::CLONE_NEWNS | libc::CLONE_NEWNET) };

                if unshare_result < 0 {
                    eprintln!("Could not unshare! {:?}", std::io::Error::last_os_error());
                    std::process::exit(2);
                }

                unsafe {
                    libc::sem_post(unshare_semaphore);
                }
            }

            println!("child: Waiting for link");
            // 18: ip link set downloader.1 netns downloader
            unsafe {
                libc::sem_wait(movelink_semaphore);
            }
            println!("child: Got link! Executing");

            // 41: ip netns exec downloader bash
            {
                // TODO: remount /sys

                let argv: Vec<*const std::ffi::c_char> = args
                    .program_args
                    .iter()
                    .map(|s| s.as_ptr() as *const i8)
                    .chain(Some(std::ptr::null()))
                    .collect();

                let env_vars: Vec<std::ffi::CString> = std::env::vars()
                    .map(|(key, value)| {
                        std::ffi::CString::new(format!("{}={}", key, value)).unwrap()
                    })
                    .collect();
                let envp: Vec<*const std::ffi::c_char> = env_vars
                    .iter()
                    .map(|s| s.as_ptr())
                    .chain(Some(std::ptr::null()))
                    .collect();

                unsafe {
                    libc::execve(
                        args.program.as_ptr() as *const i8,
                        argv.as_ptr(),
                        envp.as_ptr(),
                    );
                }
            }
        }
        // Parent
        1.. => {
            // 16: ip netns add downloader
            println!("parent: Waiting for child unshare...");
            unsafe {
                libc::sem_wait(unshare_semaphore);
            };
            println!("parent: Got child unshare!");

            // 18: ip link set downloader.1 netns downloader
            {
                unsafe {
                    libc::sem_post(movelink_semaphore);
                }
            }

            println!("parent: Signalled ready to exec");

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

    // 45: If a source IP is specified
    match &args.source_ip {
        None => {
            // 46: iptables -t nat -D POSTROUTING $(iptables --line-numbers -vn -t nat -L POSTROUTING | awk '/MASQUERADE/ { print $1 }')
        }
        Some(ip) => {
            // 48: iptables -t nat -D POSTROUTING $(iptables --line-numbers -vn -t nat -L POSTROUTING | awk '/'$1'/ { print $1 }')
        }
    }

    Ok(())
}

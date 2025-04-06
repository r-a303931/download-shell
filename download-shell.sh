#!/usr/bin/env bash

if [[ "$EUID" != "0"  ]]; then
    echo "Run this script as root"
    exit 1
fi

if ip netns | grep -q downloader; then
    echo "There is already a downloader shell running"
    exit 1
fi

echo "Sending traffic out as ${1}..."

ip link add downloader.0 type veth peer name downloader.1
ip netns add downloader
ip link set downloader.0 up
ip link set downloader.1 netns downloader

ip addr add 172.31.254.253/30 dev downloader.0

ip -n downloader link set lo up
ip -n downloader link set downloader.1 up
ip -n downloader addr add 172.31.254.254/30 dev downloader.1
ip -n downloader route add default via 172.31.254.253

DEFAULT_IF="$(ip r | grep default | sed -nE 's/^.*dev ([^ ]*) ?.*/\1/p')"
IP_FORWARD=$(cat /proc/sys/net/ipv4/ip_forward)
echo 1 > /proc/sys/net/ipv4/ip_forward

if [[ -z "$1" ]]; then
    iptables -t nat -A POSTROUTING -o "$DEFAULT_IF" -j MASQUERADE
else
    iptables -t nat -A POSTROUTING -s 172.31.254.254 -j SNAT --to-source $1

    echo 1 > /proc/sys/net/ipv4/conf/all/proxy_arp
    echo 1 > /proc/sys/net/ipv4/conf/$DEFAULT_IF/proxy_arp
    ip route add $1/32 dev downloader.0
fi

ip netns exec downloader bash

ip netns delete downloader
echo -n "$IP_FORWARD" > /proc/sys/net/ipv4/ip_forward
if [[ -z "$1" ]]; then
    iptables -t nat -D POSTROUTING $(iptables --line-numbers -vn -t nat -L POSTROUTING | awk '/MASQUERADE/ { print $1 }')
else
    iptables -t nat -D POSTROUTING $(iptables --line-numbers -vn -t nat -L POSTROUTING | awk '/'$1'/ { print $1 }')
fi

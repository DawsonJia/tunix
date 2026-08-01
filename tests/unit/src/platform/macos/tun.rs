use std::{mem, net::Ipv6Addr};

use super::{ipv4_alias_request, ipv6_alias_request, utun_unit};
use crate::{Error, NetworkAddress};

#[test]
fn utun_unit_is_checked_and_one_based() {
    assert_eq!(utun_unit("utun0").unwrap(), 1);
    assert!(matches!(
        utun_unit("utun4294967295"),
        Err(Error::InvalidName)
    ));
}

#[test]
fn ipv6_alias_without_peer_uses_unspecified_address_family() {
    let address = NetworkAddress::new("fd00::1".parse().unwrap(), 64).unwrap();
    let request = ipv6_alias_request("utun0", address).unwrap();

    assert_eq!(request.ifra_addr.sin6_family, libc::AF_INET6 as _);
    assert_eq!(request.ifra_dstaddr.sin6_len, 0);
    assert_eq!(request.ifra_dstaddr.sin6_family, libc::AF_UNSPEC as _);
    assert_eq!(
        request.ifra_dstaddr.sin6_addr.s6_addr,
        Ipv6Addr::UNSPECIFIED.octets()
    );
}

#[test]
fn alias_requests_encode_peers_in_their_address_family() {
    let ipv4_address = NetworkAddress::new("192.0.2.1".parse().unwrap(), 24)
        .unwrap()
        .with_peer("192.0.2.2".parse().unwrap())
        .unwrap();
    let ipv4 = ipv4_alias_request("utun0", ipv4_address).unwrap();
    let ipv4_peer: libc::sockaddr_in = unsafe { mem::transmute(ipv4.ifra_broadaddr) };
    assert_eq!(ipv4_peer.sin_family, libc::AF_INET as _);
    assert_eq!(ipv4_peer.sin_addr.s_addr.to_ne_bytes(), [192, 0, 2, 2]);

    let ipv6_address = NetworkAddress::new("fd00::1".parse().unwrap(), 128)
        .unwrap()
        .with_peer("fd00::2".parse().unwrap())
        .unwrap();
    let ipv6 = ipv6_alias_request("utun0", ipv6_address).unwrap();
    assert_eq!(ipv6.ifra_dstaddr.sin6_family, libc::AF_INET6 as _);
    assert_eq!(
        ipv6.ifra_dstaddr.sin6_addr.s6_addr,
        "fd00::2".parse::<Ipv6Addr>().unwrap().octets()
    );
}

#[test]
fn ipv6_alias_rejects_non_host_prefix_with_peer() {
    let address = NetworkAddress::new("fd00::1".parse().unwrap(), 64)
        .unwrap()
        .with_peer("fd00::2".parse().unwrap())
        .unwrap();
    let result = ipv6_alias_request("utun0", address);

    assert!(matches!(result, Err(Error::InvalidAddress)));
}

#[test]
fn ipv4_alias_requires_peer() {
    let address = NetworkAddress::new("192.0.2.1".parse().unwrap(), 24).unwrap();

    assert!(matches!(
        ipv4_alias_request("utun0", address),
        Err(Error::InvalidConfig)
    ));
}

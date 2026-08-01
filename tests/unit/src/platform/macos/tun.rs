use std::{mem, net::Ipv6Addr};

use super::{
    Tun, ipv4_alias_request, ipv4_ifreq, ipv4_netmask, ipv6_alias_request, utun_unit,
    validate_addresses,
};
use crate::{Configuration, Error, NetworkAddress};

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
fn ipv4_alias_request_rejects_peerless_address() {
    let address = NetworkAddress::new("192.0.2.1".parse().unwrap(), 24).unwrap();

    assert!(matches!(
        ipv4_alias_request("utun0", address),
        Err(Error::InvalidConfig)
    ));
}

#[test]
fn address_preflight_accepts_one_peerless_primary() {
    let peerful = NetworkAddress::new("192.0.2.2".parse().unwrap(), 32)
        .unwrap()
        .with_peer("192.0.2.3".parse().unwrap())
        .unwrap();
    let peerless = NetworkAddress::new("10.64.0.1".parse().unwrap(), 10).unwrap();

    assert_eq!(
        validate_addresses(&[peerful, peerless], None).unwrap(),
        Some(peerless)
    );
}

#[test]
fn address_preflight_rejects_peer_collision_in_any_order() {
    let peerless = NetworkAddress::new("10.64.0.1".parse().unwrap(), 10).unwrap();
    let peerful = NetworkAddress::new("10.64.0.1".parse().unwrap(), 32)
        .unwrap()
        .with_peer("10.64.0.2".parse().unwrap())
        .unwrap();

    for addresses in [[peerless, peerful], [peerful, peerless]] {
        assert!(matches!(
            validate_addresses(&addresses, None),
            Err(Error::InvalidConfig)
        ));
    }

    assert_eq!(validate_addresses(&[peerful, peerful], None).unwrap(), None);
}

#[test]
fn address_preflight_rejects_multiple_peerless_addresses() {
    let first = NetworkAddress::new("10.64.0.1".parse().unwrap(), 10).unwrap();
    let second = NetworkAddress::new("10.64.0.2".parse().unwrap(), 10).unwrap();

    assert!(matches!(
        validate_addresses(&[first, second], None),
        Err(Error::InvalidConfig)
    ));
}

#[test]
fn multiple_peerless_addresses_are_rejected_before_opening_utun() {
    let mut config = Configuration::default();
    config.address("10.64.0.1", 10).address("10.64.0.2", 10);

    let error = match Tun::new(&config) {
        Ok(_) => panic!("multiple peerless IPv4 addresses unexpectedly passed preflight"),
        Err(error) => error,
    };

    assert!(matches!(error, Error::InvalidConfig));
}

#[test]
fn address_preflight_rejects_ipv4_zero_prefix() {
    let peerless = NetworkAddress::new("10.64.0.1".parse().unwrap(), 0).unwrap();
    let peerful = NetworkAddress::new("192.0.2.1".parse().unwrap(), 0)
        .unwrap()
        .with_peer("192.0.2.2".parse().unwrap())
        .unwrap();

    for address in [peerless, peerful] {
        assert!(matches!(
            validate_addresses(&[address], None),
            Err(Error::InvalidAddress)
        ));
    }
}

#[test]
fn address_preflight_checks_the_configured_peerless_primary() {
    let primary = NetworkAddress::new("10.64.0.1".parse().unwrap(), 10).unwrap();
    let alias = NetworkAddress::new("192.0.2.1".parse().unwrap(), 32)
        .unwrap()
        .with_peer("192.0.2.2".parse().unwrap())
        .unwrap();
    let colliding = NetworkAddress::new("10.64.0.1".parse().unwrap(), 32)
        .unwrap()
        .with_peer("10.64.0.2".parse().unwrap())
        .unwrap();

    assert!(matches!(
        validate_addresses(&[alias, colliding], Some(primary)),
        Err(Error::InvalidConfig)
    ));
}

#[test]
fn ipv4_ifreq_encodes_non_classful_netmask() {
    let request = ipv4_ifreq("utun0", ipv4_netmask(10)).unwrap();
    let mask: libc::sockaddr_in = unsafe { mem::transmute(request.ifr_ifru.ifru_addr) };

    assert_eq!(mask.sin_family, libc::AF_INET as _);
    assert_eq!(mask.sin_addr.s_addr.to_ne_bytes(), [255, 192, 0, 0]);
}

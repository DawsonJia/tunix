use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

use tunix::{Error, IntoIpAddr, NetworkAddress};

#[test]
fn accepts_prefixes_for_both_address_families() {
    let ipv4 = NetworkAddress::new(Ipv4Addr::new(192, 0, 2, 1).into(), 24).unwrap();
    let ipv6 = NetworkAddress::new(Ipv6Addr::LOCALHOST.into(), 128).unwrap();

    assert_eq!(ipv4.ip(), IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)));
    assert_eq!(ipv4.prefix_len(), 24);
    assert_eq!(ipv4.peer(), None);
    assert_eq!(ipv6.ip(), IpAddr::V6(Ipv6Addr::LOCALHOST));
    assert_eq!(ipv6.prefix_len(), 128);
}

#[test]
fn accepts_same_family_peers() {
    let peer = Ipv4Addr::new(192, 0, 2, 2);
    let address = NetworkAddress::new(Ipv4Addr::new(192, 0, 2, 1).into(), 32)
        .unwrap()
        .with_peer(peer.into())
        .unwrap();

    assert_eq!(address.peer(), Some(peer.into()));
}

#[test]
fn rejects_mixed_family_peers() {
    let result = NetworkAddress::new(Ipv4Addr::LOCALHOST.into(), 32)
        .unwrap()
        .with_peer(Ipv6Addr::LOCALHOST.into());

    assert!(matches!(result, Err(Error::InvalidAddress)));
}

#[test]
fn converts_family_specific_socket_addresses() {
    let ipv4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 80);
    let ipv6 = SocketAddrV6::new(Ipv6Addr::LOCALHOST, 80, 0, 0);

    assert_eq!(
        ipv4.into_ip_addr().unwrap(),
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    );
    assert_eq!(
        ipv6.into_ip_addr().unwrap(),
        IpAddr::V6(Ipv6Addr::LOCALHOST)
    );
}

#[test]
fn rejects_prefixes_wider_than_the_address_family() {
    assert!(matches!(
        NetworkAddress::new(Ipv4Addr::LOCALHOST.into(), 33),
        Err(Error::InvalidAddress)
    ));
    assert!(matches!(
        NetworkAddress::new(Ipv6Addr::LOCALHOST.into(), 129),
        Err(Error::InvalidAddress)
    ));
}

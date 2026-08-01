use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use tunix::{Error, NetworkAddress};

#[test]
fn accepts_prefixes_for_both_address_families() {
    let ipv4 = NetworkAddress::new(Ipv4Addr::new(192, 0, 2, 1).into(), 24).unwrap();
    let ipv6 = NetworkAddress::new(Ipv6Addr::LOCALHOST.into(), 128).unwrap();

    assert_eq!(ipv4.ip(), IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)));
    assert_eq!(ipv4.prefix_len(), 24);
    assert_eq!(ipv6.ip(), IpAddr::V6(Ipv6Addr::LOCALHOST));
    assert_eq!(ipv6.prefix_len(), 128);
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

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use netlink_packet_route::address::AddressAttribute;

use super::address_message;
use crate::NetworkAddress;

#[test]
fn maps_local_and_optional_peer_without_family_branches() {
    let ipv4_local = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
    let ipv4_peer = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2));
    let ipv4 = NetworkAddress::new(ipv4_local, 32)
        .unwrap()
        .with_peer(ipv4_peer)
        .unwrap();
    let ipv6_local = IpAddr::V6(Ipv6Addr::LOCALHOST);
    let ipv6 = NetworkAddress::new(ipv6_local, 64).unwrap();

    for (address, local, peer) in [
        (ipv4, ipv4_local, ipv4_peer),
        (ipv6, ipv6_local, ipv6_local),
    ] {
        let message = address_message(7, address);

        assert_eq!(message.header.index, 7);
        assert_eq!(message.header.prefix_len, address.prefix_len());
        assert_eq!(
            message.attributes,
            [
                AddressAttribute::Address(peer),
                AddressAttribute::Local(local),
            ]
        );
    }
}

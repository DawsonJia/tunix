use std::net::IpAddr;

use super::NetworkAddress;

#[test]
fn maps_onto_the_routex_address_of_an_interface() {
    let ipv4: IpAddr = "192.0.2.1".parse().unwrap();
    let ipv6: IpAddr = "fd00::1".parse().unwrap();

    for (ip, prefix_len) in [(ipv4, 24), (ipv6, 64)] {
        let address = NetworkAddress::new(ip, prefix_len).unwrap();

        assert_eq!(address.on(7), routex::Address::new(7, ip, prefix_len));
    }
}

#[test]
#[cfg(not(target_os = "windows"))]
fn maps_the_peer_onto_the_routex_address() {
    let ip: IpAddr = "192.0.2.1".parse().unwrap();
    let peer: IpAddr = "192.0.2.2".parse().unwrap();
    let address = NetworkAddress::new(ip, 32)
        .unwrap()
        .with_peer(peer)
        .unwrap();

    assert_eq!(address.on(7), routex::Address::new(7, ip, 32).peer(peer));
}

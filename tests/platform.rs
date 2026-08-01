use tunix::{Configuration, interface::Interface};

#[cfg(target_os = "macos")]
use std::{ffi::CStr, net::Ipv4Addr, process::Command, ptr};
#[cfg(target_os = "macos")]
use tunix::{Error, NetworkAddress};

#[cfg(target_os = "macos")]
fn sockaddr_ipv4(address: *const libc::sockaddr) -> Option<Ipv4Addr> {
    if address.is_null() {
        return None;
    }

    unsafe {
        if (*address).sa_family != libc::AF_INET as _ {
            return None;
        }
    }

    sockaddr_ipv4_value(address)
}

#[cfg(target_os = "macos")]
fn sockaddr_ipv4_value(address: *const libc::sockaddr) -> Option<Ipv4Addr> {
    if address.is_null() {
        return None;
    }

    let address_offset = std::mem::offset_of!(libc::sockaddr_in, sin_addr);
    let address_len = usize::from(unsafe { (*address).sa_len });
    if address_len <= address_offset {
        return None;
    }

    let mut octets = [0; 4];
    let octet_count = (address_len - address_offset).min(octets.len());
    unsafe {
        ptr::copy_nonoverlapping(
            address.cast::<u8>().add(address_offset),
            octets.as_mut_ptr(),
            octet_count,
        );
    }

    Some(Ipv4Addr::from(octets))
}

#[cfg(target_os = "macos")]
fn ipv4_addresses(name: &str) -> Vec<(Ipv4Addr, Ipv4Addr, Option<Ipv4Addr>)> {
    let mut head = ptr::null_mut();
    assert_eq!(unsafe { libc::getifaddrs(&mut head) }, 0);

    let mut addresses = Vec::new();
    let mut current = head;
    while !current.is_null() {
        unsafe {
            let interface = &*current;
            if CStr::from_ptr(interface.ifa_name).to_bytes() == name.as_bytes()
                && let (Some(address), Some(netmask)) = (
                    sockaddr_ipv4(interface.ifa_addr),
                    // XNU leaves the primary IPv4 mask family as AF_UNSPEC and trims trailing
                    // zero octets, so decode it in the context of the AF_INET address.
                    sockaddr_ipv4_value(interface.ifa_netmask),
                )
            {
                addresses.push((address, netmask, sockaddr_ipv4(interface.ifa_dstaddr)));
            }
            current = interface.ifa_next;
        }
    }

    unsafe { libc::freeifaddrs(head) };
    addresses
}

#[cfg(target_os = "macos")]
fn route_interface(address: &str) -> Option<String> {
    let output = Command::new("/sbin/route")
        .args(["-n", "get", address])
        .output()
        .unwrap();
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.trim().strip_prefix("interface:").map(str::trim))
        .map(str::to_owned)
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires /dev/net/tun and CAP_NET_ADMIN"]
fn create_for_linux() {
    let mut config = Configuration::default();

    let mut dev = config
        .name("tun0")
        .address_with_peer("192.168.50.1", 32, "192.168.50.2")
        .address_with_peer("fd00::1", 128, "fd00::2")
        .mtu(1400)
        .build()
        .unwrap();

    assert_eq!("tun0", dev.name().unwrap());

    dev.set_name("tun9").unwrap();
    assert_eq!("tun9", dev.name().unwrap());
    assert_eq!(1400, dev.mtu().unwrap());
}

#[test]
#[cfg(target_os = "macos")]
#[ignore = "requires root privileges"]
fn create_for_macos() {
    let mut config = Configuration::default();

    let mut dev = config
        .address_with_peer("192.0.2.1", 32, "192.0.2.2")
        .address("10.64.0.1", 10)
        .address("fd00::1", 64)
        .mtu(1400)
        .up()
        .build()
        .unwrap();

    let name = dev.name().unwrap();
    let addresses = ipv4_addresses(&name);
    let ifconfig = Command::new("/sbin/ifconfig").arg(&name).output().unwrap();
    let ifconfig = String::from_utf8_lossy(&ifconfig.stdout);
    assert!(
        addresses.iter().any(
            |(address, netmask, _)| *address == Ipv4Addr::new(10, 64, 0, 1)
                && *netmask == Ipv4Addr::new(255, 192, 0, 0)
        ),
        "unexpected IPv4 state for {name}: {addresses:#?}\n{ifconfig}"
    );
    assert!(addresses.contains(&(
        Ipv4Addr::new(192, 0, 2, 1),
        Ipv4Addr::new(255, 255, 255, 255),
        Some(Ipv4Addr::new(192, 0, 2, 2)),
    )));
    assert_ne!(route_interface("10.64.0.2").as_deref(), Some(name.as_str()));

    let primary = NetworkAddress::new("10.64.0.1".parse().unwrap(), 10).unwrap();
    dev.add_address(primary).unwrap();

    let colliding = NetworkAddress::new("10.64.0.1".parse().unwrap(), 32)
        .unwrap()
        .with_peer("10.64.0.2".parse().unwrap())
        .unwrap();
    assert!(matches!(
        dev.add_address(colliding),
        Err(Error::InvalidConfig)
    ));
    assert!(ipv4_addresses(&name).contains(&(
        Ipv4Addr::new(10, 64, 0, 1),
        Ipv4Addr::new(255, 192, 0, 0),
        None,
    )));
    assert_ne!(route_interface("10.64.0.2").as_deref(), Some(name.as_str()));

    let second = NetworkAddress::new("10.64.0.2".parse().unwrap(), 10).unwrap();
    assert!(matches!(dev.add_address(second), Err(Error::InvalidConfig)));
    assert!(
        ipv4_addresses(&name)
            .iter()
            .any(
                |(address, netmask, _)| *address == Ipv4Addr::new(10, 64, 0, 1)
                    && *netmask == Ipv4Addr::new(255, 192, 0, 0)
            )
    );
    assert_eq!(1400, dev.mtu().unwrap());
}

#[test]
#[cfg(target_os = "macos")]
#[ignore = "requires root privileges"]
fn peerless_ipv4_cannot_replace_peerful_primary() {
    let mut dev = Configuration::default().build().unwrap();
    let peerful = NetworkAddress::new("192.0.2.1".parse().unwrap(), 32)
        .unwrap()
        .with_peer("192.0.2.2".parse().unwrap())
        .unwrap();
    dev.add_address(peerful).unwrap();

    let peerless = NetworkAddress::new("10.64.0.1".parse().unwrap(), 10).unwrap();
    assert!(matches!(
        dev.add_address(peerless),
        Err(Error::InvalidConfig)
    ));

    let addresses = ipv4_addresses(&dev.name().unwrap());
    assert!(
        addresses
            .iter()
            .any(|(address, _, peer)| *address == Ipv4Addr::new(192, 0, 2, 1)
                && *peer == Some(Ipv4Addr::new(192, 0, 2, 2)))
    );
}

#[test]
#[cfg(target_os = "windows")]
#[ignore = "requires Administrator privileges and Wintun"]
fn create_for_windows() {
    let mut config = Configuration::default();

    let dev = config
        .name("tunix-test")
        .address("192.168.50.1", 24)
        .address("fd00::1", 64)
        .build()
        .unwrap();

    assert_eq!("tunix-test", dev.name().unwrap());
}

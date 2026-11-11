use tunix::{Configuration, interface::Interface};

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "macos")]
use std::{ffi::CStr, net::Ipv4Addr, ptr};
#[cfg(unix)]
use std::{io, process::Command};
#[cfg(unix)]
use tunix::Error;
#[cfg(target_os = "macos")]
use tunix::NetworkAddress;

/// A request that `routex` refused comes back as its `io::Error`.
#[cfg(unix)]
fn assert_io_error(result: tunix::Result<()>, kind: io::ErrorKind) {
    assert!(
        matches!(&result, Err(Error::Io(error)) if error.kind() == kind),
        "{result:?} is not {kind:?}"
    );
}

/// The addresses of the interface `name`, one a line, as `ip address` shows them.
#[cfg(target_os = "linux")]
fn ip_addresses(name: &str) -> String {
    let output = Command::new("ip")
        .args(["-o", "address", "show", "dev", name])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");

    String::from_utf8(output.stdout).unwrap()
}

/// An attribute of the interface `name`, as sysfs has it.
#[cfg(target_os = "linux")]
fn sysfs(name: &str, attribute: &str) -> String {
    fs::read_to_string(format!("/sys/class/net/{name}/{attribute}"))
        .unwrap()
        .trim()
        .to_owned()
}

#[cfg(target_os = "linux")]
fn is_up(name: &str) -> bool {
    let flags = sysfs(name, "flags");
    let flags = u32::from_str_radix(flags.trim_start_matches("0x"), 16).unwrap();

    flags & libc::IFF_UP as u32 != 0
}

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
        .address("10.64.0.1", 10)
        .mtu(1400)
        .up()
        .build()
        .unwrap();

    assert_eq!("tun0", dev.name().unwrap());
    assert_eq!(routex::if_nametoindex("tun0"), Some(dev.index()));

    let addresses = ip_addresses("tun0");
    for address in [
        "inet 192.168.50.1 peer 192.168.50.2/32",
        "inet6 fd00::1 peer fd00::2/128",
        "inet 10.64.0.1/10",
    ] {
        assert!(addresses.contains(address), "no {address} in:\n{addresses}");
    }
    assert_eq!(1400, dev.mtu().unwrap());
    assert_eq!("1400", sysfs("tun0", "mtu"));
    assert!(is_up("tun0"));

    // An address that is there is no error, and one the kernel refuses is that of routex.
    let address = "10.64.0.1".parse().unwrap();
    dev.add_address(tunix::NetworkAddress::new(address, 10).unwrap())
        .unwrap();
    assert_io_error(dev.set_mtu(u32::MAX), io::ErrorKind::InvalidInput);
    assert_eq!("1400", sysfs("tun0", "mtu"));

    dev.enable(false).unwrap();
    assert!(!is_up("tun0"));

    // The interface keeps its index under another name.
    dev.set_name("tun9").unwrap();
    assert_eq!("tun9", dev.name().unwrap());
    assert_eq!(routex::if_nametoindex("tun9"), Some(dev.index()));
    dev.set_mtu(1300).unwrap();
    assert_eq!(1300, dev.mtu().unwrap());
    assert_eq!("1300", sysfs("tun9", "mtu"));
    dev.enable(true).unwrap();
    assert!(is_up("tun9"));
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "requires /dev/net/tun and CAP_NET_ADMIN"]
fn create_multi_queue_for_linux() {
    let mut devs = Configuration::default()
        .name("tun1")
        .address("10.65.0.1", 24)
        .address("fd00:1::1", 64)
        .mtu(1400)
        .queues(2)
        .up()
        .build_multi_queue()
        .unwrap();

    assert_eq!(2, devs.len());
    let addresses = ip_addresses("tun1");
    for address in ["inet 10.65.0.1/24", "inet6 fd00:1::1/64"] {
        assert!(addresses.contains(address), "no {address} in:\n{addresses}");
    }
    assert!(is_up("tun1"));

    // The queues are of one interface: each configures it.
    let index = routex::if_nametoindex("tun1");
    for dev in &mut devs {
        assert_eq!("tun1", dev.name().unwrap());
        assert_eq!(index, Some(dev.index()));
        assert_eq!(1400, dev.mtu().unwrap());
    }
    devs[1].set_mtu(1300).unwrap();
    assert_eq!(1300, devs[0].mtu().unwrap());
}

#[test]
#[cfg(target_os = "macos")]
#[ignore = "requires root privileges"]
fn create_for_macos() {
    let mut config = Configuration::default();

    // The IPv4 address without a peer goes before the one with a peer.
    let mut dev = config
        .address("10.64.0.1", 10)
        .address_with_peer("192.0.2.1", 32, "192.0.2.2")
        .address("fd00::1", 64)
        .mtu(1400)
        .up()
        .build()
        .unwrap();

    let name = dev.name().unwrap();
    assert_eq!(routex::if_nametoindex(&name), Some(dev.index()));

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
    for state in ["<UP,", "mtu 1400", "inet6 fd00::1 prefixlen 64"] {
        assert!(ifconfig.contains(state), "no {state} in:\n{ifconfig}");
    }
    // The address without a peer brings no route to its prefix.
    assert_ne!(route_interface("10.64.0.2").as_deref(), Some(name.as_str()));

    // An address that is there is no error.
    let primary = NetworkAddress::new("10.64.0.1".parse().unwrap(), 10).unwrap();
    dev.add_address(primary).unwrap();

    // A second address without a peer would take the place of the first.
    let second = NetworkAddress::new("10.64.0.2".parse().unwrap(), 10).unwrap();
    assert_io_error(dev.add_address(second), io::ErrorKind::InvalidInput);
    assert!(
        ipv4_addresses(&name)
            .iter()
            .any(
                |(address, netmask, _)| *address == Ipv4Addr::new(10, 64, 0, 1)
                    && *netmask == Ipv4Addr::new(255, 192, 0, 0)
            )
    );
    assert_eq!(1400, dev.mtu().unwrap());

    dev.enable(false).unwrap();
    let ifconfig = Command::new("/sbin/ifconfig").arg(&name).output().unwrap();
    assert!(!String::from_utf8_lossy(&ifconfig.stdout).contains("<UP,"));
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
    assert_io_error(dev.add_address(peerless), io::ErrorKind::InvalidInput);

    let addresses = ipv4_addresses(&dev.name().unwrap());
    assert!(
        addresses
            .iter()
            .any(|(address, _, peer)| *address == Ipv4Addr::new(192, 0, 2, 1)
                && *peer == Some(Ipv4Addr::new(192, 0, 2, 2)))
    );
}

#[test]
#[cfg(target_os = "macos")]
#[ignore = "requires root privileges"]
fn build_fails_on_the_address_macos_refuses() {
    // The addresses are set in the order they were added, so the one without a peer comes
    // too late here.
    let result = Configuration::default()
        .address_with_peer("192.0.2.1", 32, "192.0.2.2")
        .address("10.64.0.1", 10)
        .build();

    assert_io_error(result.map(drop), io::ErrorKind::InvalidInput);
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
        .mtu(1400)
        .build()
        .unwrap();

    assert_eq!("tunix-test", dev.name().unwrap());
    assert_eq!(
        routex::if_friendly_name_to_index("tunix-test"),
        Some(dev.index())
    );
    assert_eq!(1400, dev.mtu().unwrap());
}

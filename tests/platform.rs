use tunix::{Configuration, interface::Interface};

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

    let dev = config
        .name("utun6")
        .address_with_peer("192.168.50.1", 32, "192.168.50.2")
        .address("fd00::1", 64)
        .mtu(1400)
        .up()
        .build()
        .unwrap();

    assert_eq!("utun6", dev.name().unwrap());
    assert_eq!(1400, dev.mtu().unwrap());
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

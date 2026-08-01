use tunix::{Configuration, interface::Interface};

#[test]
#[cfg(target_os = "linux")]
fn create_for_linux() {
    let mut config = Configuration::default();

    let mut dev = config
        .name("tun0")
        .address("192.168.50.1", 24)
        .address("fd00::1", 64)
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
fn create_for_macos() {
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("skipping macOS TUN create test: root privileges required");
        return;
    }

    let mut config = Configuration::default();

    let dev = config
        .name("utun6")
        .address("192.168.50.1", 24)
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
#[ignore]
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

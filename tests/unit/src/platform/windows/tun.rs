use super::Tun;
use crate::{Configuration, Error};

#[test]
fn peer_is_rejected_before_loading_wintun() {
    let mut config = Configuration::default();
    config
        .address_with_peer("192.0.2.1", 32, "192.0.2.2")
        .platform(|platform| {
            platform.wintun_file(r"Z:\tunix-preflight-test\missing-wintun.dll");
        });

    let error = match Tun::new(&config) {
        Ok(_) => panic!("point-to-point peer unexpectedly passed Windows preflight"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        Error::Io(error) if error.kind() == std::io::ErrorKind::Unsupported
    ));
}

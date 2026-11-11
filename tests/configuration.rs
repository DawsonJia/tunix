use std::net::{Ipv4Addr, Ipv6Addr};

use tunix::{Configuration, NetworkAddress, Result, interface::Interface};

#[derive(Default)]
struct RecordingInterface {
    addresses: Vec<NetworkAddress>,
    #[cfg(not(target_os = "windows"))]
    enabled: bool,
    mtu: u32,
    queue: (),
}

impl Interface for RecordingInterface {
    type Queue = ();

    fn name(&self) -> Result<String> {
        Ok("recording".into())
    }

    fn set_name(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }

    fn index(&self) -> u32 {
        0
    }

    #[cfg(not(target_os = "windows"))]
    fn enable(&mut self, value: bool) -> Result<()> {
        self.enabled = value;
        Ok(())
    }

    fn add_address(&mut self, address: NetworkAddress) -> Result<()> {
        self.addresses.push(address);
        Ok(())
    }

    fn mtu(&self) -> Result<u32> {
        Ok(self.mtu)
    }

    fn set_mtu(&mut self, mtu: u32) -> Result<()> {
        self.mtu = mtu;
        Ok(())
    }

    fn queue(&mut self) -> &mut Self::Queue {
        &mut self.queue
    }
}

#[test]
fn configures_addresses_in_the_order_they_were_added() {
    let mut config = Configuration::default();
    config
        .address("fd00::1", 64)
        .address("192.0.2.1", 24)
        .mtu(1280);

    let mut interface = RecordingInterface::default();
    interface.configure(&config).unwrap();

    assert_eq!(
        interface.addresses,
        [
            NetworkAddress::new(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1).into(), 64).unwrap(),
            NetworkAddress::new(Ipv4Addr::new(192, 0, 2, 1).into(), 24).unwrap(),
        ]
    );
    assert_eq!(interface.mtu, 1280);
}

#[test]
#[cfg(not(target_os = "windows"))]
fn configures_a_peer_and_the_state_of_the_link() {
    let mut config = Configuration::default();
    config.address_with_peer("192.0.2.1", 32, "192.0.2.2");

    let mut interface = RecordingInterface {
        enabled: true,
        ..Default::default()
    };
    interface.configure(&config).unwrap();
    assert!(!interface.enabled);

    interface.configure(config.up()).unwrap();
    assert!(interface.enabled);

    let address = NetworkAddress::new(Ipv4Addr::new(192, 0, 2, 1).into(), 32)
        .unwrap()
        .with_peer(Ipv4Addr::new(192, 0, 2, 2).into())
        .unwrap();
    assert_eq!(interface.addresses, [address, address]);
}

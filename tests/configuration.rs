use std::net::{Ipv4Addr, Ipv6Addr};

use tunix::{Configuration, NetworkAddress, Result, interface::Interface};

#[derive(Default)]
struct RecordingInterface {
    addresses: Vec<NetworkAddress>,
    enabled: bool,
    mtu: i32,
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

    fn enable(&mut self, value: bool) -> Result<()> {
        self.enabled = value;
        Ok(())
    }

    fn flags(&self, flags: Option<i16>) -> Result<i16> {
        Ok(flags.unwrap_or_default())
    }

    fn add_address(&mut self, address: NetworkAddress) -> Result<()> {
        self.addresses.push(address);
        Ok(())
    }

    fn mtu(&self) -> Result<i32> {
        Ok(self.mtu)
    }

    fn set_mtu(&mut self, mtu: i32) -> Result<()> {
        self.mtu = mtu;
        Ok(())
    }

    fn queue(&mut self) -> &mut Self::Queue {
        &mut self.queue
    }
}

#[test]
fn configures_ipv4_and_ipv6_through_one_interface_path() {
    let mut config = Configuration::default();
    config
        .address("192.0.2.1", 24)
        .address("fd00::1", 64)
        .mtu(1280)
        .up();

    let mut interface = RecordingInterface::default();
    interface.configure(&config).unwrap();

    assert_eq!(
        interface.addresses,
        [
            NetworkAddress::new(Ipv4Addr::new(192, 0, 2, 1).into(), 24).unwrap(),
            NetworkAddress::new(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1).into(), 64).unwrap(),
        ]
    );
    assert_eq!(interface.mtu, 1280);
    assert!(interface.enabled);
}

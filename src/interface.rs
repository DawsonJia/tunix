use crate::NetworkAddress;
use crate::configuration::Configuration;
use crate::error::*;

pub trait Interface /*: Read + Write*/ {
    // type Queue: Read + Write;
    type Queue;

    fn configure(&mut self, config: &Configuration) -> Result<()> {
        self.add_addresses(&config.addresses)?;

        if let Some(mtu) = config.mtu {
            self.set_mtu(mtu)?;
        }

        self.enable(config.enabled)?;

        Ok(())
    }

    fn name(&self) -> Result<String>;
    fn set_name(&mut self, name: &str) -> Result<()>;

    fn enable(&mut self, value: bool) -> Result<()>;
    fn flags(&self, flags: Option<i16>) -> Result<i16>;
    // fn set_enabled(&mut self, value: bool) -> Result<()>;

    /// Adds either an IPv4 or IPv6 address and its optional point-to-point peer.
    ///
    /// macOS supports one peerless IPv4 primary, rejects IPv4 `/0`, and does not allow a different
    /// peerless IPv4 to replace the primary or a peerful IPv4 to reuse its local address.
    fn add_address(&mut self, address: NetworkAddress) -> Result<()>;

    /// Adds a batch of interface addresses.
    ///
    /// Platforms may validate and order the batch before applying any address.
    fn add_addresses(&mut self, addresses: &[NetworkAddress]) -> Result<()> {
        for address in addresses {
            self.add_address(*address)?;
        }

        Ok(())
    }

    fn mtu(&self) -> Result<i32>;
    fn set_mtu(&mut self, mtu: i32) -> Result<()>;

    fn queue(&mut self) -> &mut Self::Queue;
}

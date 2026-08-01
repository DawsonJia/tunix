use crate::NetworkAddress;
use crate::configuration::Configuration;
use crate::error::*;

pub trait Interface /*: Read + Write*/ {
    // type Queue: Read + Write;
    type Queue;

    fn configure(&mut self, config: &Configuration) -> Result<()> {
        for address in &config.addresses {
            self.add_address(*address)?;
        }

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

    /// Adds either an IPv4 or IPv6 address through the platform-native API.
    fn add_address(&mut self, address: NetworkAddress) -> Result<()>;

    fn mtu(&self) -> Result<i32>;
    fn set_mtu(&mut self, mtu: i32) -> Result<()>;

    fn queue(&mut self) -> &mut Self::Queue;
}

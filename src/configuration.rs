#[cfg(all(
    feature = "async",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
use crate::AsyncTun;
use crate::address::{IntoIpAddr, NetworkAddress};
use crate::error::{Error, Result};
use crate::tun::{Tun, TunConf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layer {
    L2,
    #[default]
    L3,
}

#[derive(Debug, Default, Clone)]
pub struct Configuration {
    pub(crate) name: Option<String>,
    pub(crate) platform: TunConf,

    pub(crate) addresses: Vec<NetworkAddress>,
    // Keep invalid address input sticky so a later setter call cannot silently hide it.
    pub(crate) invalid_address: bool,
    pub(crate) mtu: Option<i32>,
    // Set the interface to be enabled once crated
    pub(crate) enabled: bool,
    pub(crate) layer: Layer,
    pub(crate) queues: Option<usize>,
}

impl Configuration {
    pub fn platform<F: FnOnce(&mut TunConf)>(&mut self, f: F) -> &mut Self {
        f(&mut self.platform);
        self
    }

    pub fn name<S: AsRef<str>>(&mut self, name: S) -> &mut Self {
        self.name = Some(name.as_ref().into());
        self
    }

    /// Adds an interface address.
    ///
    /// Call this once for each address family to configure a dual-stack interface.
    ///
    /// ```
    /// let mut config = tunix::Configuration::default();
    /// config
    ///     .address("192.0.2.1", 24)
    ///     .address("fd00::1", 64);
    /// ```
    pub fn address<A: IntoIpAddr>(&mut self, value: A, prefix_len: u8) -> &mut Self {
        match value
            .into_ip_addr()
            .and_then(|ip| NetworkAddress::new(ip, prefix_len))
        {
            Ok(address) => self.addresses.push(address),
            Err(_) => self.invalid_address = true,
        }

        self
    }

    /// Adds already-validated interface addresses.
    pub fn addresses<I>(&mut self, values: I) -> &mut Self
    where
        I: IntoIterator<Item = NetworkAddress>,
    {
        self.addresses.extend(values);
        self
    }

    pub fn mtu(&mut self, value: i32) -> &mut Self {
        self.mtu = Some(value);
        self
    }

    pub fn up(&mut self) -> &mut Self {
        self.enabled = true;
        self
    }

    pub fn down(&mut self) -> &mut Self {
        self.enabled = false;
        self
    }

    pub fn layer(&mut self, layer: Layer) -> &mut Self {
        self.layer = layer;
        self
    }

    pub fn queues(&mut self, queues: usize) -> &mut Self {
        self.queues = Some(queues);
        self
    }

    pub fn build(&self) -> Result<Tun> {
        self.ensure_single_queue()?;
        Tun::new(self)
    }

    #[cfg(target_os = "linux")]
    pub fn build_multi_queue(&self) -> Result<Vec<Tun>> {
        self.ensure_valid()?;

        Tun::new_multi_queue(self)
    }

    #[cfg(all(
        feature = "async",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    pub fn build_async(&self) -> Result<AsyncTun> {
        self.ensure_single_queue()?;

        AsyncTun::new(Tun::new(self)?)
    }

    #[cfg(all(feature = "async", target_os = "linux"))]
    pub fn build_async_multi_queue(&self) -> Result<Vec<AsyncTun>> {
        self.ensure_valid()?;

        AsyncTun::new_multi_queue(Tun::new_multi_queue(self)?)
    }

    fn ensure_valid(&self) -> Result<()> {
        if self.invalid_address {
            return Err(Error::InvalidAddress);
        }
        if self.queues == Some(0) {
            return Err(Error::InvalidQueuesNumber);
        }

        Ok(())
    }

    fn ensure_single_queue(&self) -> Result<()> {
        self.ensure_valid()?;
        if self.queues.unwrap_or(1) > 1 {
            return Err(Error::InvalidQueuesNumber);
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/unit/src/configuration.rs"]
mod tests;

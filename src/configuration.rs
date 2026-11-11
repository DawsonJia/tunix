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
    pub(crate) mtu: Option<u32>,
    // Set the interface to be enabled once crated
    #[cfg(not(target_os = "windows"))]
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
    /// Call this once for each address family to configure a dual-stack interface. The
    /// addresses are set in the order they were added, each as [`Interface::add_address`] sets
    /// it: one that the interface has already is no error. One that the platform refuses fails
    /// the build with [`Error::Io`]. A build is no transaction: an interface that it made goes
    /// with it, but one that was there before, a persistent TUN of Linux or an adapter of
    /// Windows, keeps what the build set on it before it failed.
    ///
    /// - macOS has no prefix `/0`. A utun is point-to-point, where an IPv4 address without a
    ///   peer is the first IPv4 address of the interface or none: add at most one, before the
    ///   IPv4 addresses with a peer. It brings no route: the caller adds the one to its prefix.
    /// - Windows: an adapter of the configured name that is there already is opened with the
    ///   addresses it has, and one of them that is added again takes the new prefix.
    ///
    /// [`Interface::add_address`]: crate::interface::Interface::add_address
    ///
    /// ```
    /// let mut config = tunix::Configuration::default();
    /// config
    ///     .address("192.0.2.1", 24)
    ///     .address("fd00::1", 64);
    /// ```
    pub fn address<A: IntoIpAddr>(&mut self, value: A, prefix_len: u8) -> &mut Self {
        let address = value
            .into_ip_addr()
            .and_then(|ip| NetworkAddress::new(ip, prefix_len));

        self.push_address(address)
    }

    /// Adds an interface address with a point-to-point peer, set as those of
    /// [`address`](Self::address) are.
    ///
    /// With a host prefix (`/32` or `/128`), which is the portable choice, the address brings
    /// a route to the peer and to no other host. Linux routes a shorter prefix as well: that
    /// of the peer for an IPv4 address, and for an IPv6 one its own, beside the route to the
    /// peer.
    ///
    /// Not on Windows, which has no peers. macOS has no prefix `/0`, and takes the peer of an
    /// IPv6 address under prefix `/128` only, and only a peer that the routing table has no
    /// host route to yet.
    ///
    /// ```
    /// let mut config = tunix::Configuration::default();
    /// config.address_with_peer("192.0.2.1", 32, "192.0.2.2");
    /// ```
    #[cfg(not(target_os = "windows"))]
    pub fn address_with_peer<A: IntoIpAddr, P: IntoIpAddr>(
        &mut self,
        value: A,
        prefix_len: u8,
        peer: P,
    ) -> &mut Self {
        let address = value.into_ip_addr().and_then(|ip| {
            let peer = peer.into_ip_addr()?;
            NetworkAddress::new(ip, prefix_len)?.with_peer(peer)
        });

        self.push_address(address)
    }

    /// Adds structurally validated interface addresses, set after those added before them, as
    /// those of [`address`](Self::address) are.
    pub fn addresses<I>(&mut self, values: I) -> &mut Self
    where
        I: IntoIterator<Item = NetworkAddress>,
    {
        self.addresses.extend(values);
        self
    }

    fn push_address(&mut self, address: Result<NetworkAddress>) -> &mut Self {
        match address {
            Ok(address) => self.addresses.push(address),
            Err(_) => self.invalid_address = true,
        }

        self
    }

    /// Sets the MTU of the interface. Without it the interface keeps the MTU the platform gave
    /// it.
    ///
    /// An MTU the interface does not take is the platform's to refuse, and fails the build.
    /// Windows has an MTU for each of IPv4 and IPv6 and sets both, or neither for an MTU under
    /// the 1280 that IPv6 takes.
    pub fn mtu(&mut self, value: u32) -> &mut Self {
        self.mtu = Some(value);
        self
    }

    /// Brings the interface up once its addresses and MTU are set.
    ///
    /// Not on Windows: a Wintun adapter is up for as long as its session, which is as long as
    /// its `Tun`.
    #[cfg(not(target_os = "windows"))]
    pub fn up(&mut self) -> &mut Self {
        self.enabled = true;
        self
    }

    /// Leaves the interface down, as it is unless [`up`](Self::up) was called. Not on Windows.
    #[cfg(not(target_os = "windows"))]
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

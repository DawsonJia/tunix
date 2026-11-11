use routex::Handle;

use crate::NetworkAddress;
use crate::configuration::Configuration;
use crate::error::*;

/// A network interface: its name, its packet queue, and the addresses, MTU and state that
/// [`routex`] sets on it by its [`index`](Interface::index).
///
/// What each platform makes of an address, an MTU and a link is in the
/// [Semantics of `routex`](https://docs.rs/routex/0.5/routex/#semantics). A request that the
/// platform refuses fails with the [`Error::Io`] of `routex`.
pub trait Interface /*: Read + Write*/ {
    // type Queue: Read + Write;
    type Queue;

    /// Applies `config`: its addresses in the order they were added, then its MTU, then, where
    /// the platform has it, whether the interface is up.
    fn configure(&mut self, config: &Configuration) -> Result<()> {
        for address in &config.addresses {
            self.add_address(*address)?;
        }

        if let Some(mtu) = config.mtu {
            self.set_mtu(mtu)?;
        }

        #[cfg(not(target_os = "windows"))]
        self.enable(config.enabled)?;

        Ok(())
    }

    fn name(&self) -> Result<String>;
    fn set_name(&mut self, name: &str) -> Result<()>;

    /// The index of the interface. `routex` takes an interface by its index, for its routes
    /// as well as for its addresses and link. A renamed interface keeps its index.
    fn index(&self) -> u32;

    /// Brings the interface up, or down, as [`routex::Handle::set_link_up`] does.
    ///
    /// Not on Windows: a Wintun adapter is up for as long as its session, which is as long as
    /// its `Tun`.
    #[cfg(not(target_os = "windows"))]
    fn enable(&mut self, value: bool) -> Result<()> {
        Ok(Handle::new()?.set_link_up(self.index(), value)?)
    }

    /// Makes `address` an address of the interface, whether or not it is one already, as
    /// `ip addr replace` and [`routex::Handle::replace_address`] do.
    ///
    /// - Linux: an IPv4 address is its ip, prefix and peer, so with another prefix or peer it
    ///   is added beside the one that is there. An IPv6 ip is there once, and keeps the prefix
    ///   it was added with.
    /// - macOS: an ip is there once. An IPv4 address that is there takes the new prefix and
    ///   peer; an IPv6 one with another prefix or peer fails with `AlreadyExists`. No address
    ///   has prefix `/0`. On a utun, which is point-to-point, an IPv4 address without a peer is
    ///   the first IPv4 address of the interface or none: it fails with `InvalidInput` where it
    ///   would take the place of another, and with `AlreadyExists` where it is there with a
    ///   peer. It brings no route: the caller adds the one to its prefix. An IPv6 address
    ///   takes a peer under prefix `/128` only, and only a peer that the routing table has no
    ///   host route to yet, or it fails with `AlreadyExists`.
    /// - Windows: an ip is there once, and takes the new prefix. There are no peers.
    fn add_address(&mut self, address: NetworkAddress) -> Result<()> {
        Ok(Handle::new()?.replace_address(&address.on(self.index()))?)
    }

    /// The MTU of the interface, as [`routex::Handle::get_link_mtu`] reads it: on Windows that
    /// of IPv4, or of IPv6 on an adapter without IPv4.
    fn mtu(&self) -> Result<u32> {
        Ok(Handle::new()?.get_link_mtu(self.index())?)
    }

    /// Sets the MTU of the interface, as [`routex::Handle::set_link_mtu`] does. An MTU the
    /// interface does not take is the platform's to refuse. Windows has one for each of IPv4
    /// and IPv6 and sets both, or neither for an MTU under the 1280 that IPv6 takes.
    fn set_mtu(&mut self, mtu: u32) -> Result<()> {
        Ok(Handle::new()?.set_link_mtu(self.index(), mtu)?)
    }

    fn queue(&mut self) -> &mut Self::Queue;
}

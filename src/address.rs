use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use crate::error::{Error, Result};

/// An IP address of an interface, with the point-to-point peer it may have on Linux and macOS.
///
/// Windows has no peers, so `with_peer` and `peer` do not exist there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkAddress {
    ip: IpAddr,
    prefix_len: u8,
    #[cfg(not(target_os = "windows"))]
    peer: Option<IpAddr>,
}

impl NetworkAddress {
    /// Creates an interface address after validating the prefix for its address family.
    pub fn new(ip: IpAddr, prefix_len: u8) -> Result<Self> {
        let max_prefix_len = if ip.is_ipv4() { 32 } else { 128 };
        if prefix_len > max_prefix_len {
            return Err(Error::InvalidAddress);
        }

        Ok(Self {
            ip,
            prefix_len,
            #[cfg(not(target_os = "windows"))]
            peer: None,
        })
    }

    /// Sets the address at the other end of a point-to-point link. The address then brings a
    /// route to the peer, not to its prefix.
    ///
    /// The peer must use the same address family as the local address. What else a platform
    /// asks of a peer is checked when the address is applied: macOS takes the peer of an IPv6
    /// address under prefix `/128` only. A host prefix (`/32` or `/128`) is the portable choice.
    #[cfg(not(target_os = "windows"))]
    pub fn with_peer(mut self, peer: IpAddr) -> Result<Self> {
        if self.ip.is_ipv4() != peer.is_ipv4() {
            return Err(Error::InvalidAddress);
        }

        self.peer = Some(peer);
        Ok(self)
    }

    /// Returns the IP address.
    pub fn ip(self) -> IpAddr {
        self.ip
    }

    /// Returns the CIDR prefix length.
    pub fn prefix_len(self) -> u8 {
        self.prefix_len
    }

    /// Returns the point-to-point peer address, if one was configured.
    #[cfg(not(target_os = "windows"))]
    pub fn peer(self) -> Option<IpAddr> {
        self.peer
    }

    /// This address on the interface `ifindex`, as `routex` takes it.
    pub(crate) fn on(self, ifindex: u32) -> routex::Address {
        routex::Address {
            ifindex,
            ip: self.ip,
            prefix: self.prefix_len,
            #[cfg(not(target_os = "windows"))]
            peer: self.peer,
        }
    }
}

/// Converts common address representations into [`IpAddr`].
pub trait IntoIpAddr {
    /// Performs the conversion.
    fn into_ip_addr(self) -> Result<IpAddr>;
}

impl IntoIpAddr for u32 {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok(Ipv4Addr::from(self).into())
    }
}

impl IntoIpAddr for i32 {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok(Ipv4Addr::from(self as u32).into())
    }
}

impl IntoIpAddr for (u8, u8, u8, u8) {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok(Ipv4Addr::new(self.0, self.1, self.2, self.3).into())
    }
}

impl IntoIpAddr for &str {
    fn into_ip_addr(self) -> Result<IpAddr> {
        self.parse().map_err(|_| Error::InvalidAddress)
    }
}

impl IntoIpAddr for String {
    fn into_ip_addr(self) -> Result<IpAddr> {
        self.as_str().into_ip_addr()
    }
}

impl IntoIpAddr for &String {
    fn into_ip_addr(self) -> Result<IpAddr> {
        self.as_str().into_ip_addr()
    }
}

impl IntoIpAddr for Ipv4Addr {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok(self.into())
    }
}

impl IntoIpAddr for &Ipv4Addr {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok((*self).into())
    }
}

impl IntoIpAddr for Ipv6Addr {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok(self.into())
    }
}

impl IntoIpAddr for &Ipv6Addr {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok((*self).into())
    }
}

impl IntoIpAddr for IpAddr {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok(self)
    }
}

impl IntoIpAddr for &IpAddr {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok(*self)
    }
}

impl IntoIpAddr for SocketAddr {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok(self.ip())
    }
}

impl IntoIpAddr for &SocketAddr {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok(self.ip())
    }
}

impl IntoIpAddr for SocketAddrV4 {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok((*self.ip()).into())
    }
}

impl IntoIpAddr for &SocketAddrV4 {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok((*self.ip()).into())
    }
}

impl IntoIpAddr for SocketAddrV6 {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok((*self.ip()).into())
    }
}

impl IntoIpAddr for &SocketAddrV6 {
    fn into_ip_addr(self) -> Result<IpAddr> {
        Ok((*self.ip()).into())
    }
}

#[cfg(test)]
#[path = "../tests/unit/src/address.rs"]
mod tests;

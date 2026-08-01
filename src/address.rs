use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::error::{Error, Result};

/// An IP address assigned to a network interface and its on-link prefix length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkAddress {
    ip: IpAddr,
    prefix_len: u8,
}

impl NetworkAddress {
    /// Creates an interface address after validating the prefix for its address family.
    pub fn new(ip: IpAddr, prefix_len: u8) -> Result<Self> {
        let max_prefix_len = if ip.is_ipv4() { 32 } else { 128 };
        if prefix_len > max_prefix_len {
            return Err(Error::InvalidAddress);
        }

        Ok(Self { ip, prefix_len })
    }

    /// Returns the IP address.
    pub fn ip(self) -> IpAddr {
        self.ip
    }

    /// Returns the CIDR prefix length.
    pub fn prefix_len(self) -> u8 {
        self.prefix_len
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

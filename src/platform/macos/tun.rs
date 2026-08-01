use std::ffi::CStr;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::fd::{AsRawFd, RawFd};
use std::{io, mem};

use crate::Configuration;
use crate::configuration::Layer;
use crate::error::{Error, Result};
use crate::interface::Interface;
use crate::platform::posix::sys::cvt;
use crate::platform::posix::{fd::Fd, name::write_if_name};
use libc::{c_char, c_uchar, c_uint, socklen_t};

use super::sys::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct TunConf {}

pub struct Queue {
    tun: Fd,
}

impl Queue {
    pub fn has_packet_information(&self) -> bool {
        // alway true for macos
        true
    }

    fn set_nonblocking(&self) -> io::Result<()> {
        self.tun.set_nonblocking(true)
    }

    fn cancel_nonblocking(&self) -> io::Result<()> {
        self.tun.set_nonblocking(false)
    }
}

impl AsRawFd for Queue {
    fn as_raw_fd(&self) -> RawFd {
        self.tun.as_raw_fd()
    }
}

pub struct Tun {
    pub(crate) name: String,
    pub(crate) queue: Queue,
    pub(crate) ctl: Fd,
    pub(crate) ctl_v6: Fd,
}

fn utun_unit(name: &str) -> Result<u32> {
    if name.len() >= libc::IFNAMSIZ {
        return Err(Error::NameTooLong);
    }
    let index = name
        .strip_prefix("utun")
        .ok_or(Error::InvalidName)?
        .parse::<u32>()?;

    index.checked_add(1).ok_or(Error::InvalidName)
}

impl Tun {
    pub fn new(config: &Configuration) -> Result<Self> {
        let id = if let Some(name) = config.name.as_ref() {
            // The control API uses one-based units while interface names use a
            // zero-based suffix (utun0 maps to sc_unit 1).
            utun_unit(name)?
        } else {
            0u32
        };

        if config.layer != Layer::L3 {
            return Err(Error::UnsupportedLayer);
        }

        let queue_number = config.queues.unwrap_or(1);
        if queue_number != 1 {
            return Err(Error::InvalidQueuesNumber);
        }

        let tun_fd = Fd::socket(libc::AF_SYSTEM, libc::SOCK_DGRAM, libc::SYSPROTO_CONTROL)?;

        // get ctl id with utun control name
        let mut info: libc::ctl_info = unsafe { std::mem::zeroed() };
        UTUN_CONTROL_NAME
            .bytes()
            .zip(info.ctl_name.iter_mut())
            .for_each(|(b, ptr)| *ptr = b as c_char);
        info.ctl_id = 0;
        // SAFETY: info is a writable ctl_info value for the duration of ioctl.
        cvt(unsafe {
            libc::ioctl(
                tun_fd.as_raw_fd(),
                libc::CTLIOCGINFO,
                &mut info as *mut _ as *mut libc::c_void,
            )
        })?;

        // connect to sys control interface
        let mut addr: libc::sockaddr_ctl = unsafe { std::mem::zeroed() };
        addr.sc_len = mem::size_of::<libc::sockaddr_ctl>() as c_uchar;
        addr.sc_family = libc::AF_SYSTEM as c_uchar;
        addr.ss_sysaddr = libc::AF_SYS_CONTROL as _;
        addr.sc_id = info.ctl_id;
        addr.sc_unit = id as c_uint;
        // addr.sc_reserved = [0; 5];
        // SAFETY: addr points to a fully initialized sockaddr_ctl of the supplied length.
        cvt(unsafe {
            libc::connect(
                tun_fd.as_raw_fd(),
                &addr as *const libc::sockaddr_ctl as *const libc::sockaddr,
                mem::size_of::<libc::sockaddr_ctl>() as libc::socklen_t,
            )
        })?;

        // get interface name
        let mut ifname = [0i8; libc::IFNAMSIZ];
        let mut len = ifname.len() as socklen_t;
        // SAFETY: ifname is writable for len bytes and len has the expected socklen_t type.
        cvt(unsafe {
            libc::getsockopt(
                tun_fd.as_raw_fd(),
                libc::SYSPROTO_CONTROL,
                libc::UTUN_OPT_IFNAME,
                ifname.as_mut_ptr() as *mut libc::c_void,
                &mut len,
            )
        })?;

        // new a control fd
        let ctl = Fd::socket(libc::AF_INET, libc::SOCK_DGRAM, 0)?;
        let ctl_v6 = Fd::socket(libc::AF_INET6, libc::SOCK_DGRAM, 0)?;

        let mut tun = Self {
            name: unsafe {
                CStr::from_ptr(ifname.as_ptr())
                    .to_string_lossy()
                    .to_string()
            },
            queue: Queue { tun: tun_fd },
            ctl,
            ctl_v6,
        };

        tun.configure(config)?;

        Ok(tun)
    }

    fn ifreq(&self) -> libc::ifreq {
        let mut ifr: libc::ifreq = unsafe { std::mem::zeroed() };

        write_if_name(&self.name, &mut ifr.ifr_name).expect("stored interface name must be valid");

        ifr
    }

    fn set_alias_v4(&mut self, address: crate::NetworkAddress) -> Result<()> {
        let ifar = ipv4_alias_request(&self.name, address)?;

        unsafe { siocaifaddr(self.ctl.as_raw_fd(), &ifar) }?;

        Ok(())
    }

    fn set_alias_v6(&mut self, address: crate::NetworkAddress) -> Result<()> {
        let ifar = ipv6_alias_request(&self.name, address)?;

        unsafe { siocaifaddr_in6(self.ctl_v6.as_raw_fd(), &ifar) }?;

        Ok(())
    }

    pub fn has_packet_information(&self) -> bool {
        self.queue.has_packet_information()
    }

    pub fn layer(&self) -> Layer {
        Layer::L3
    }

    pub fn set_nonblocking(&self) -> io::Result<()> {
        self.queue.set_nonblocking()
    }

    pub fn cancel_nonblocking(&self) -> io::Result<()> {
        self.queue.cancel_nonblocking()
    }
}

impl AsRawFd for Tun {
    fn as_raw_fd(&self) -> RawFd {
        self.queue.tun.as_raw_fd()
    }
}

impl Interface for Tun {
    type Queue = Queue;

    fn name(&self) -> Result<String> {
        Ok(self.name.clone())
    }
    // can not set interface name on Darwin
    fn set_name(&mut self, _name: &str) -> Result<()> {
        Err(Error::InvalidName)
    }

    fn enable(&mut self, value: bool) -> Result<()> {
        let mut flags = self.flags(None)?;

        if value {
            flags |= libc::IFF_UP as i16 | libc::IFF_RUNNING as i16;
        } else {
            flags &= !(libc::IFF_UP as i16);
        }

        self.flags(Some(flags))?;

        Ok(())
    }
    fn flags(&self, flags: Option<i16>) -> Result<i16> {
        let mut ifr = self.ifreq();

        if let Some(flags) = flags {
            ifr.ifr_ifru.ifru_flags = flags;
            unsafe { siocsifflags(self.ctl.as_raw_fd(), &ifr) }?;
        } else {
            unsafe { siocgifflags(self.ctl.as_raw_fd(), &mut ifr) }?;
        }

        Ok(unsafe { ifr.ifr_ifru.ifru_flags })
    }

    fn add_address(&mut self, address: crate::NetworkAddress) -> Result<()> {
        match address.ip() {
            IpAddr::V4(_) => self.set_alias_v4(address),
            IpAddr::V6(_) => self.set_alias_v6(address),
        }
    }

    fn mtu(&self) -> Result<i32> {
        let mut ifr = self.ifreq();
        unsafe { siocgifmtu(self.ctl.as_raw_fd(), &mut ifr) }?;

        Ok(unsafe { ifr.ifr_ifru.ifru_mtu })
    }

    fn set_mtu(&mut self, mtu: i32) -> Result<()> {
        let mut ifr = self.ifreq();
        ifr.ifr_ifru.ifru_mtu = mtu;

        unsafe { siocsifmtu(self.ctl.as_raw_fd(), &ifr) }?;

        Ok(())
    }

    fn queue(&mut self) -> &mut Self::Queue {
        &mut self.queue
    }
}

fn ipv4_sockaddr(addr: Ipv4Addr) -> libc::sockaddr {
    let mut socket_addr: libc::sockaddr_in = unsafe { mem::zeroed() };
    socket_addr.sin_len = mem::size_of::<libc::sockaddr_in>() as u8;
    socket_addr.sin_family = libc::AF_INET as _;
    socket_addr.sin_addr = libc::in_addr {
        s_addr: u32::from_ne_bytes(addr.octets()),
    };

    unsafe { mem::transmute(socket_addr) }
}

fn ipv6_sockaddr(addr: Ipv6Addr) -> libc::sockaddr_in6 {
    let mut socket_addr: libc::sockaddr_in6 = unsafe { mem::zeroed() };
    socket_addr.sin6_len = mem::size_of::<libc::sockaddr_in6>() as u8;
    socket_addr.sin6_family = libc::AF_INET6 as _;
    socket_addr.sin6_addr = libc::in6_addr {
        s6_addr: addr.octets(),
    };

    socket_addr
}

fn ipv4_alias_request(name: &str, address: crate::NetworkAddress) -> Result<ifaliasreq> {
    let Some(peer) = address.peer() else {
        // TODO: Support peerless IPv4 only when configuration owns an explicit route policy.
        return Err(Error::InvalidConfig);
    };
    let (IpAddr::V4(addr), IpAddr::V4(peer)) = (address.ip(), peer) else {
        return Err(Error::InvalidAddress);
    };

    let mut ifar: ifaliasreq = unsafe { mem::zeroed() };
    write_if_name(name, &mut ifar.ifra_name)?;

    ifar.ifra_addr = ipv4_sockaddr(addr);
    ifar.ifra_broadaddr = ipv4_sockaddr(peer);
    ifar.ifra_mask = ipv4_sockaddr(ipv4_netmask(address.prefix_len()));

    Ok(ifar)
}

fn ipv6_alias_request(name: &str, address: crate::NetworkAddress) -> Result<in6_aliasreq> {
    let IpAddr::V6(addr) = address.ip() else {
        return Err(Error::InvalidAddress);
    };
    let peer = match address.peer() {
        Some(IpAddr::V6(peer)) => Some(peer),
        Some(IpAddr::V4(_)) => return Err(Error::InvalidAddress),
        None => None,
    };
    if peer.is_some() && address.prefix_len() != 128 {
        return Err(Error::InvalidAddress);
    }

    let mut ifar: in6_aliasreq = unsafe { mem::zeroed() };
    write_if_name(name, &mut ifar.ifra_name)?;

    ifar.ifra_addr = ipv6_sockaddr(addr);
    if let Some(peer) = peer {
        ifar.ifra_dstaddr = ipv6_sockaddr(peer);
    }
    ifar.ifra_prefixmask = ipv6_sockaddr(ipv6_netmask(address.prefix_len()));
    ifar.ifra_lifetime.ia6t_vltime = u32::MAX;
    ifar.ifra_lifetime.ia6t_pltime = u32::MAX;

    Ok(ifar)
}

fn ipv4_netmask(prefix_len: u8) -> Ipv4Addr {
    Ipv4Addr::from(
        u32::MAX
            .checked_shl(32 - u32::from(prefix_len))
            .unwrap_or(0),
    )
}

fn ipv6_netmask(prefix_len: u8) -> Ipv6Addr {
    Ipv6Addr::from(
        u128::MAX
            .checked_shl(128 - u32::from(prefix_len))
            .unwrap_or(0),
    )
}

impl Read for Tun {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.queue.tun.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        self.queue.tun.read_vectored(bufs)
    }
}

impl Write for Tun {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.queue.tun.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.queue.tun.flush()
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.queue.tun.write_vectored(bufs)
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/src/platform/macos/tun.rs"]
mod tests;

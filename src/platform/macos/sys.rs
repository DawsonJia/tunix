use libc::ifreq;

pub const UTUN_CONTROL_NAME: &str = "com.apple.net.utun_control";

// ifaliasreq can not be found from libc
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ifaliasreq {
    pub ifra_name: [libc::c_char; libc::IFNAMSIZ],
    pub ifra_addr: libc::sockaddr,
    pub ifra_broadaddr: libc::sockaddr,
    pub ifra_mask: libc::sockaddr,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct in6_addrlifetime {
    pub ia6t_expire: libc::time_t,
    pub ia6t_preferred: libc::time_t,
    pub ia6t_vltime: u32,
    pub ia6t_pltime: u32,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct in6_aliasreq {
    pub ifra_name: [libc::c_char; libc::IFNAMSIZ],
    pub ifra_addr: libc::sockaddr_in6,
    pub ifra_dstaddr: libc::sockaddr_in6,
    pub ifra_prefixmask: libc::sockaddr_in6,
    pub ifra_flags: libc::c_int,
    pub ifra_lifetime: in6_addrlifetime,
}

const _: [(); 128] = [(); std::mem::size_of::<in6_aliasreq>()];

nix::ioctl_write_ptr!(siocsifflags, b'i', 16, ifreq);
nix::ioctl_readwrite!(siocgifflags, b'i', 17, ifreq);

nix::ioctl_write_ptr!(siocsifaddr, b'i', 12, ifreq);
nix::ioctl_write_ptr!(siocsifnetmask, b'i', 22, ifreq);
nix::ioctl_write_ptr!(siocdifaddr, b'i', 25, ifreq);

nix::ioctl_write_ptr!(siocsifmtu, b'i', 52, ifreq);
nix::ioctl_readwrite!(siocgifmtu, b'i', 51, ifreq);

// SIOCAIFADDR
nix::ioctl_write_ptr!(siocaifaddr, b'i', 26, ifaliasreq);
nix::ioctl_write_ptr!(siocaifaddr_in6, b'i', 26, in6_aliasreq);

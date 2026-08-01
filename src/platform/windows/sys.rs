use std::{io, net::IpAddr};

use windows::Win32::{
    Foundation::BOOLEAN,
    NetworkManagement::{
        IpHelper::{
            CreateUnicastIpAddressEntry, DeleteUnicastIpAddressEntry, GetIpInterfaceEntry,
            GetUnicastIpAddressEntry, InitializeIpInterfaceEntry, InitializeUnicastIpAddressEntry,
            MIB_IPINTERFACE_ROW, MIB_UNICASTIPADDRESS_ROW, SetIpInterfaceEntry,
        },
        Ndis::NET_LUID_LH,
    },
    Networking::WinSock::{
        AF_INET, AF_INET6, IpDadStatePreferred, IpPrefixOriginManual, IpSuffixOriginManual,
        SOCKADDR_INET,
    },
};

use crate::NetworkAddress;

pub fn set_mtu(luid: NET_LUID_LH, mtu: i32) -> io::Result<()> {
    let mtu = mtu
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "MTU must be non-negative"))?;

    for family in [AF_INET, AF_INET6] {
        let mut row = MIB_IPINTERFACE_ROW::default();
        unsafe {
            InitializeIpInterfaceEntry(&mut row);
        }
        row.Family = family;
        row.InterfaceLuid = luid;

        unsafe {
            GetIpInterfaceEntry(&mut row).map_err(io::Error::other)?;
        }
        row.NlMtu = mtu;
        // GetIpInterfaceEntry can return a row whose SitePrefixLength is not
        // valid input for SetIpInterfaceEntry.
        row.SitePrefixLength = 0;
        unsafe {
            SetIpInterfaceEntry(&mut row).map_err(io::Error::other)?;
        }
    }

    Ok(())
}

pub fn add_address(luid: NET_LUID_LH, address: NetworkAddress) -> io::Result<()> {
    let socket_address = socket_address(address.ip());

    let mut probe = MIB_UNICASTIPADDRESS_ROW::default();
    unsafe {
        InitializeUnicastIpAddressEntry(&mut probe);
    }
    probe.InterfaceLuid = luid;
    probe.Address = socket_address;

    if unsafe { GetUnicastIpAddressEntry(&mut probe) }.is_ok() {
        unsafe {
            DeleteUnicastIpAddressEntry(&probe).map_err(io::Error::other)?;
        }
    }

    let mut row = MIB_UNICASTIPADDRESS_ROW::default();
    unsafe {
        InitializeUnicastIpAddressEntry(&mut row);
    }
    row.InterfaceLuid = luid;
    row.Address = socket_address;
    row.OnLinkPrefixLength = address.prefix_len();
    row.PrefixOrigin = IpPrefixOriginManual;
    row.SuffixOrigin = IpSuffixOriginManual;
    row.ValidLifetime = u32::MAX;
    row.PreferredLifetime = u32::MAX;
    row.SkipAsSource = BOOLEAN(0);
    row.DadState = IpDadStatePreferred;

    unsafe { CreateUnicastIpAddressEntry(&row).map_err(io::Error::other) }
}

fn socket_address(ip: IpAddr) -> SOCKADDR_INET {
    let mut socket_address = SOCKADDR_INET::default();
    match ip {
        IpAddr::V4(ip) => {
            socket_address.Ipv4.sin_family = AF_INET;
            socket_address.Ipv4.sin_addr.S_un.S_addr = u32::from_ne_bytes(ip.octets());
        }
        IpAddr::V6(ip) => {
            socket_address.Ipv6.sin6_family = AF_INET6;
            socket_address.Ipv6.sin6_addr.u.Byte = ip.octets();
        }
    }

    socket_address
}

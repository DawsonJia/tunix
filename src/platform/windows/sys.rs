use std::{io, net::IpAddr};

use windows::Win32::{
    Foundation::{BOOLEAN, ERROR_NOT_FOUND},
    NetworkManagement::{
        IpHelper::{
            CreateUnicastIpAddressEntry, GetIpInterfaceEntry, GetUnicastIpAddressEntry,
            InitializeIpInterfaceEntry, InitializeUnicastIpAddressEntry, MIB_IPINTERFACE_ROW,
            MIB_UNICASTIPADDRESS_ROW, SetIpInterfaceEntry, SetUnicastIpAddressEntry,
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

    let mut updated = false;
    for family in [AF_INET, AF_INET6] {
        updated |= set_family_mtu(luid, family, mtu)?;
    }

    if updated {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(ERROR_NOT_FOUND.0 as i32))
    }
}

pub fn add_address(luid: NET_LUID_LH, address: NetworkAddress) -> io::Result<()> {
    if address.peer().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "point-to-point peers are not supported on Windows",
        ));
    }

    let mut row = address_identity(luid, address.ip());
    match unsafe { GetUnicastIpAddressEntry(&mut row) } {
        Ok(()) => {
            apply_address_properties(&mut row, address);
            unsafe { SetUnicastIpAddressEntry(&row).map_err(io::Error::other) }
        }
        Err(error) if is_not_found(&error) => {
            let mut row = address_identity(luid, address.ip());
            apply_new_address_properties(&mut row, address);

            unsafe { CreateUnicastIpAddressEntry(&row).map_err(io::Error::other) }
        }
        Err(error) => Err(io::Error::other(error)),
    }
}

fn set_family_mtu(
    luid: NET_LUID_LH,
    family: windows::Win32::Networking::WinSock::ADDRESS_FAMILY,
    mtu: u32,
) -> io::Result<bool> {
    let mut row = MIB_IPINTERFACE_ROW::default();
    unsafe {
        InitializeIpInterfaceEntry(&mut row);
    }
    row.Family = family;
    row.InterfaceLuid = luid;

    if let Err(error) = unsafe { GetIpInterfaceEntry(&mut row) } {
        return if is_not_found(&error) {
            Ok(false)
        } else {
            Err(io::Error::other(error))
        };
    }

    row.NlMtu = mtu;
    // GetIpInterfaceEntry can return a row whose SitePrefixLength is not
    // valid input for SetIpInterfaceEntry.
    row.SitePrefixLength = 0;

    match unsafe { SetIpInterfaceEntry(&mut row) } {
        Ok(()) => Ok(true),
        Err(error) if is_not_found(&error) => Ok(false),
        Err(error) => Err(io::Error::other(error)),
    }
}

fn address_identity(luid: NET_LUID_LH, ip: IpAddr) -> MIB_UNICASTIPADDRESS_ROW {
    let mut row = MIB_UNICASTIPADDRESS_ROW::default();
    unsafe {
        InitializeUnicastIpAddressEntry(&mut row);
    }
    row.InterfaceLuid = luid;
    row.Address = socket_address(ip);

    row
}

fn apply_address_properties(row: &mut MIB_UNICASTIPADDRESS_ROW, address: NetworkAddress) {
    row.OnLinkPrefixLength = address.prefix_len();
    row.PrefixOrigin = IpPrefixOriginManual;
    row.SuffixOrigin = IpSuffixOriginManual;
    row.ValidLifetime = u32::MAX;
    row.PreferredLifetime = u32::MAX;
    row.SkipAsSource = BOOLEAN(0);
}

fn apply_new_address_properties(row: &mut MIB_UNICASTIPADDRESS_ROW, address: NetworkAddress) {
    apply_address_properties(row, address);
    // Windows 10+ makes the address usable while optimistic DAD runs.
    row.DadState = IpDadStatePreferred;
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

fn is_not_found(error: &windows::core::Error) -> bool {
    error.code() == ERROR_NOT_FOUND.to_hresult()
}

#[cfg(test)]
#[path = "../../../tests/unit/src/platform/windows/sys.rs"]
mod tests;

use std::{
    ffi::CString,
    io::{self, Read, Write},
    num::NonZeroI32,
    os::fd::AsRawFd,
};

use libc::{c_int, ifreq, sockaddr_nl};
use netlink_packet_core::{
    NLM_F_ACK, NLM_F_CREATE, NLM_F_REPLACE, NLM_F_REQUEST, NetlinkHeader, NetlinkMessage,
    NetlinkPayload,
};
use netlink_packet_route::{
    AddressFamily, RouteNetlinkMessage,
    address::{AddressAttribute, AddressMessage, AddressScope},
};

use crate::{
    NetworkAddress,
    platform::posix::{fd::Fd, sys::cvt},
};

nix::ioctl_read_bad!(siocgifflags, 0x8913, ifreq); // get flags
nix::ioctl_write_ptr_bad!(siocsifflags, 0x8914, ifreq); // set flags

nix::ioctl_read_bad!(siocgifmtu, 0x8921, ifreq); // get MTU size
nix::ioctl_write_ptr_bad!(siocsifmtu, 0x8922, ifreq); // set MTU size

nix::ioctl_read_bad!(siocgifname, 0x8910, ifreq); // get iface name
nix::ioctl_write_ptr_bad!(siocsifname, 0x8923, ifreq); // set interface name

nix::ioctl_write_ptr!(tunsetiff, b'T', 202, c_int);
nix::ioctl_write_ptr!(tunsetpersist, b'T', 203, c_int);
nix::ioctl_write_ptr!(tunsetowner, b'T', 204, c_int);
nix::ioctl_write_ptr!(tunsetgroup, b'T', 205, c_int);

pub fn add_address(interface_name: &str, address: NetworkAddress) -> io::Result<()> {
    let interface_name = CString::new(interface_name)?;
    let interface_index = unsafe { libc::if_nametoindex(interface_name.as_ptr()) };
    if interface_index == 0 {
        return Err(io::Error::last_os_error());
    }

    let mut socket = Fd::socket(libc::AF_NETLINK, libc::SOCK_RAW, libc::NETLINK_ROUTE)?;
    let mut local: sockaddr_nl = unsafe { std::mem::zeroed() };
    local.nl_family = libc::AF_NETLINK as u16;
    cvt(unsafe {
        libc::bind(
            socket.as_raw_fd(),
            (&local as *const sockaddr_nl).cast(),
            std::mem::size_of::<sockaddr_nl>() as _,
        )
    })?;

    let mut assigned: sockaddr_nl = unsafe { std::mem::zeroed() };
    let mut assigned_len = std::mem::size_of::<sockaddr_nl>() as libc::socklen_t;
    cvt(unsafe {
        libc::getsockname(
            socket.as_raw_fd(),
            (&mut assigned as *mut sockaddr_nl).cast(),
            &mut assigned_len,
        )
    })?;

    let message = address_message(interface_index, address);

    let sequence = 1;
    let mut header = NetlinkHeader::default();
    header.flags = NLM_F_REQUEST | NLM_F_ACK | NLM_F_CREATE | NLM_F_REPLACE;
    header.sequence_number = sequence;
    header.port_number = assigned.nl_pid;

    let mut request = NetlinkMessage::new(
        header,
        NetlinkPayload::from(RouteNetlinkMessage::NewAddress(message)),
    );
    request.finalize();
    let mut buffer = vec![0; request.buffer_len()];
    request.serialize(&mut buffer);
    socket.write_all(&buffer)?;

    recv_ack(&mut socket, sequence, assigned.nl_pid)
}

fn address_message(interface_index: u32, address: NetworkAddress) -> AddressMessage {
    let local = address.ip();
    let mut message = AddressMessage::default();
    message.header.family = AddressFamily::from(local);
    message.header.prefix_len = address.prefix_len();
    message.header.scope = AddressScope::Universe;
    message.header.index = interface_index;
    message
        .attributes
        .push(AddressAttribute::Address(address.peer().unwrap_or(local)));
    message.attributes.push(AddressAttribute::Local(local));

    message
}

fn recv_ack(socket: &mut Fd, sequence: u32, port_id: u32) -> io::Result<()> {
    const ACK_TIMEOUT: libc::timeval = libc::timeval {
        tv_sec: 5,
        tv_usec: 0,
    };

    // SAFETY: ACK_TIMEOUT points to a valid timeval for the duration of the call.
    cvt(unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            (&ACK_TIMEOUT as *const libc::timeval).cast(),
            std::mem::size_of::<libc::timeval>() as _,
        )
    })?;

    loop {
        let mut buffer = [0; 4096];
        let received = match socket.read(&mut buffer) {
            Ok(received) => received,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "timed out waiting for netlink address acknowledgement",
                ));
            }
            Err(error) => return Err(error),
        };
        for message in parse_messages(&buffer[..received])? {
            if message.header.sequence_number != sequence || message.header.port_number != port_id {
                continue;
            }

            return match message.payload {
                NetlinkPayload::Error(error) => match error.code {
                    Some(code) => Err(netlink_error(code)),
                    None => Ok(()),
                },
                payload => Err(io::Error::other(format!(
                    "unexpected netlink address response: {payload:?}"
                ))),
            };
        }
    }
}

fn parse_messages(bytes: &[u8]) -> io::Result<Vec<NetlinkMessage<RouteNetlinkMessage>>> {
    const NETLINK_HEADER_LEN: usize = 16;

    let mut messages = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let remaining = &bytes[offset..];
        if remaining.len() < NETLINK_HEADER_LEN {
            return Err(io::Error::other("truncated netlink address response"));
        }

        let message = NetlinkMessage::<RouteNetlinkMessage>::deserialize(remaining)
            .map_err(|error| io::Error::other(format!("{error:?}")))?;
        let length = message.header.length as usize;
        if length < NETLINK_HEADER_LEN || length > remaining.len() {
            return Err(io::Error::other("invalid netlink address response length"));
        }

        messages.push(message);
        offset = offset
            .checked_add((length + 3) & !3)
            .ok_or_else(|| io::Error::other("netlink address response length overflow"))?;
    }

    Ok(messages)
}

fn netlink_error(code: NonZeroI32) -> io::Error {
    let raw = code.get();
    io::Error::from_raw_os_error(if raw < 0 { -raw } else { raw })
}

#[cfg(test)]
#[path = "../../../tests/unit/src/platform/linux/sys.rs"]
mod tests;

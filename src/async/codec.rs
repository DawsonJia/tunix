use bytes::{BufMut, Bytes, BytesMut};
use std::io;
use tokio_util::codec::{Decoder, Encoder};

use crate::configuration::Layer;

const PACKET_INFORMATION_LEN: usize = 4;
const ETHERNET_HEADER_LEN: usize = 14;

#[derive(Debug, Clone, Copy, Default)]
pub enum PacketProtocol {
    #[default]
    Ipv4,
    Ipv6,
    Other(u8),
}

/// Infer the IP protocol from the first byte. Empty input is reported as `Other(0)`.
pub fn infer_proto(pkt: &[u8]) -> PacketProtocol {
    // | version: 4bits | ihl: 4 bits | service: 8 bits | total length: 16 bits
    // | identification: 16 bits | flags: 3 bits, fragment offset: 13 bits
    // | time to live: 8 bits | protocol: 8 bits | header checksum: 16 bits
    // | source address: 32 bits
    // | destination address: 32 bits
    match pkt.first().map(|byte| byte >> 4) {
        Some(4) => PacketProtocol::Ipv4,
        Some(6) => PacketProtocol::Ipv6,
        Some(p) => PacketProtocol::Other(p),
        None => PacketProtocol::Other(0),
    }
}

impl PacketProtocol {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub fn into_pi_field(self) -> Result<u16, io::Error> {
        match self {
            PacketProtocol::Ipv4 => Ok(libc::PF_INET as u16),
            PacketProtocol::Ipv6 => Ok(libc::PF_INET6 as u16),
            PacketProtocol::Other(p) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("neither an Ipv4 nor Ipv6 packet: {p}"),
            )),
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn into_pi_field(self) -> Result<u16, io::Error> {
        match self {
            PacketProtocol::Ipv4 => Ok(libc::ETH_P_IP as u16),
            PacketProtocol::Ipv6 => Ok(libc::ETH_P_IPV6 as u16),
            PacketProtocol::Other(p) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("neither an Ipv4 nor Ipv6 packet: {p}"),
            )),
        }
    }

    #[cfg(target_os = "windows")]
    pub fn into_pi_field(self) -> Result<u16, io::Error> {
        match self {
            PacketProtocol::Ipv4 => Ok(0x0800),
            PacketProtocol::Ipv6 => Ok(0x86dd),
            PacketProtocol::Other(p) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("neither an Ipv4 nor Ipv6 packet: {p}"),
            )),
        }
    }
}

pub struct TunPacket(Bytes);

impl TunPacket {
    pub fn new<T: Into<Bytes>>(pkt: T) -> Self {
        Self(pkt.into())
    }

    pub fn get_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn into_bytes(self) -> Bytes {
        self.0
    }
}

impl From<TunPacket> for Bytes {
    fn from(value: TunPacket) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TunPacketCodec {
    packet_information: bool,
    layer: Layer,
    mtu: usize,
}

impl TunPacketCodec {
    pub fn new(packet_information: bool, layer: Layer, mtu: usize) -> Self {
        Self {
            packet_information,
            layer,
            mtu,
        }
    }

    pub(crate) fn frame_capacity(&self) -> usize {
        self.mtu
            .saturating_add(match self.layer {
                Layer::L2 => ETHERNET_HEADER_LEN,
                Layer::L3 => 0,
            })
            .saturating_add(if self.packet_information {
                PACKET_INFORMATION_LEN
            } else {
                0
            })
    }

    fn packet_protocol(&self, packet: &[u8]) -> io::Result<u16> {
        match self.layer {
            Layer::L3 => infer_proto(packet).into_pi_field(),
            Layer::L2 => packet
                .get(12..14)
                .map(|field| u16::from_be_bytes([field[0], field[1]]))
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Ethernet frame is shorter than 14 bytes",
                    )
                }),
        }
    }
}

/// impl [`Decoder`] and [`Encoder`] trait for TunPacketCodec
/// the [`Framed`] will implement the [`Stream`] trait
///
/// [`Decoder`]: tokio_util::codec::Decoder
/// [`Encoder`]: tokio_util::codec::Encoder
/// [`Framed`]: tokio_util::codec::Framed
/// [`Stream`]: futures::stream::Stream
// TODO: Replace the unresolved Stream intra-doc link with a dependency-free external link.
impl Decoder for TunPacketCodec {
    type Item = TunPacket;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buf.is_empty() {
            return Ok(None);
        }

        let mut pkt = buf.split_to(buf.len());

        buf.reserve(self.frame_capacity());

        if self.packet_information {
            if pkt.len() < PACKET_INFORMATION_LEN {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "packet information header is shorter than 4 bytes",
                ));
            }

            let _ = pkt.split_to(PACKET_INFORMATION_LEN);
        }

        if pkt.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "packet payload is empty",
            ));
        }

        Ok(Some(TunPacket(pkt.freeze())))
    }
}

impl Encoder<TunPacket> for TunPacketCodec {
    type Error = io::Error;

    fn encode(&mut self, item: TunPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let TunPacket(packet) = item;
        dst.reserve(
            packet
                .len()
                .saturating_add(if self.packet_information { 4 } else { 0 }),
        );

        if self.packet_information {
            let protocol = self.packet_protocol(&packet)?;
            dst.put_u16(0);
            dst.put_u16(protocol);
        }
        dst.put(packet);

        Ok(())
    }
}

#[cfg(test)]
#[path = "../../tests/unit/src/async/codec.rs"]
mod tests;

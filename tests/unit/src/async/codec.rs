use super::{PacketProtocol, TunPacketCodec, infer_proto};
use crate::{Layer, TunPacket};
use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

#[test]
fn infer_proto_accepts_empty_packet() {
    assert!(matches!(infer_proto(&[]), PacketProtocol::Other(0)));
}

#[test]
fn decoder_rejects_short_packet_information_header() {
    let mut codec = TunPacketCodec::new(true, Layer::L3, 1500);
    let mut buf = BytesMut::from(&[0u8, 0, 8][..]);

    let result = codec.decode(&mut buf);

    assert!(matches!(
        result,
        Err(err) if err.kind() == std::io::ErrorKind::InvalidData
    ));
}

#[test]
fn frame_capacity_includes_link_and_packet_information_headers() {
    assert_eq!(
        TunPacketCodec::new(true, Layer::L2, 9000).frame_capacity(),
        9018
    );
    assert_eq!(
        TunPacketCodec::new(false, Layer::L3, 9000).frame_capacity(),
        9000
    );
}

#[test]
#[cfg(target_os = "linux")]
fn linux_tap_packet_information_uses_ethernet_type() {
    let frame = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0x08, 0x06, 0, 0, 0, 0];
    let mut encoded = BytesMut::new();
    TunPacketCodec::new(true, Layer::L2, 1500)
        .encode(TunPacket::new(frame.to_vec()), &mut encoded)
        .unwrap();

    assert_eq!(&encoded[..4], &[0, 0, 0x08, 0x06]);
    assert_eq!(&encoded[4..], frame);
}

#[test]
fn layer_two_packet_information_rejects_short_ethernet_frame() {
    let mut encoded = BytesMut::new();
    let result = TunPacketCodec::new(true, Layer::L2, 1500)
        .encode(TunPacket::new(&[0; 13][..]), &mut encoded);

    assert!(matches!(
        result,
        Err(err) if err.kind() == std::io::ErrorKind::InvalidData
    ));
}

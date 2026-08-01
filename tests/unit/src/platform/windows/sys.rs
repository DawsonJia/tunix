use windows::Win32::{
    Foundation::{ERROR_INVALID_PARAMETER, ERROR_NOT_FOUND},
    NetworkManagement::Ndis::NET_LUID_LH,
    Networking::WinSock::{
        IpDadStatePreferred, IpDadStateTentative, IpPrefixOriginManual, IpSuffixOriginManual,
    },
};

use super::{add_address, apply_address_properties, apply_new_address_properties, is_not_found};
use crate::NetworkAddress;

#[test]
fn only_error_not_found_is_skippable() {
    assert!(is_not_found(&windows::core::Error::from(ERROR_NOT_FOUND)));
    assert!(!is_not_found(&windows::core::Error::from(
        ERROR_INVALID_PARAMETER,
    )));
}

#[test]
fn updating_address_properties_preserves_dad_state() {
    let address = NetworkAddress::new("192.0.2.1".parse().unwrap(), 24).unwrap();
    let mut row = windows::Win32::NetworkManagement::IpHelper::MIB_UNICASTIPADDRESS_ROW {
        DadState: IpDadStateTentative,
        ..Default::default()
    };

    apply_address_properties(&mut row, address);

    assert_eq!(row.OnLinkPrefixLength, 24);
    assert_eq!(row.PrefixOrigin, IpPrefixOriginManual);
    assert_eq!(row.SuffixOrigin, IpSuffixOriginManual);
    assert_eq!(row.ValidLifetime, u32::MAX);
    assert_eq!(row.PreferredLifetime, u32::MAX);
    assert_eq!(row.SkipAsSource.0, 0);
    assert_eq!(row.DadState, IpDadStateTentative);
}

#[test]
fn new_addresses_use_optimistic_dad() {
    let address = NetworkAddress::new("fd00::1".parse().unwrap(), 64).unwrap();
    let mut row = Default::default();

    apply_new_address_properties(&mut row, address);

    assert_eq!(row.DadState, IpDadStatePreferred);
}

#[test]
fn point_to_point_peer_is_rejected_before_windows_api_calls() {
    let address = NetworkAddress::new("192.0.2.1".parse().unwrap(), 32)
        .unwrap()
        .with_peer("192.0.2.2".parse().unwrap())
        .unwrap();

    let error = add_address(NET_LUID_LH::default(), address).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);
}

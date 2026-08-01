use super::utun_unit;
use crate::Error;

#[test]
fn utun_unit_is_checked_and_one_based() {
    assert_eq!(utun_unit("utun0").unwrap(), 1);
    assert!(matches!(
        utun_unit("utun4294967295"),
        Err(Error::InvalidName)
    ));
}

use super::Configuration;
use crate::error::Error;

#[test]
fn invalid_address_is_reported_by_build() {
    let mut config = Configuration::default();
    let result = config.address("not an ip", 24).build();

    assert!(matches!(result, Err(Error::InvalidAddress)));
}

#[test]
fn invalid_prefix_is_reported_by_build() {
    let mut config = Configuration::default();
    let result = config.address("192.0.2.1", 33).build();

    assert!(matches!(result, Err(Error::InvalidAddress)));
}

#[test]
fn single_queue_build_rejects_zero_and_multiple_queues() {
    for queues in [0, 2] {
        let result = Configuration::default().queues(queues).build();
        assert!(matches!(result, Err(Error::InvalidQueuesNumber)));
    }
}

#[test]
#[cfg(all(
    feature = "async",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
fn single_queue_async_build_rejects_multiple_queues() {
    let result = Configuration::default().queues(2).build_async();
    assert!(matches!(result, Err(Error::InvalidQueuesNumber)));
}

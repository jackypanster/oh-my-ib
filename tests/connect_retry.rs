//! FROZEN SPEC — connect-retry (review-05 follow-up B). Offline, std-only: asserts the
//! transient-error classifier that decides whether `ib::connect` retries. The retry behavior
//! itself is gateway-dependent (live acceptance). The coder must NOT edit this file.
//! RED until impl adds `oh_my_ib::ib::is_transient_io`.

use oh_my_ib::ib::is_transient_io;
use std::io::ErrorKind;

#[test]
fn transient_kinds_are_retryable() {
    assert!(
        is_transient_io(ErrorKind::WouldBlock),
        "EAGAIN/WouldBlock (the observed back-to-back error) must be transient"
    );
    assert!(is_transient_io(ErrorKind::Interrupted));
    assert!(is_transient_io(ErrorKind::TimedOut));
}

#[test]
fn permanent_kinds_are_not_retryable() {
    assert!(
        !is_transient_io(ErrorKind::ConnectionRefused),
        "refused = gateway down, must fail fast (keeps the dead-port test fast)"
    );
    assert!(!is_transient_io(ErrorKind::NotFound));
    assert!(!is_transient_io(ErrorKind::PermissionDenied));
}

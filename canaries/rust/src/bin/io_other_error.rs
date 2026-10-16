//! Canary.
use std::io::{Error, ErrorKind};

fn main() {
    let err = Error::new(ErrorKind::Other, "boom"); //~ clippy::io_other_error
    assert!(err.get_ref().is_some(), "inner");
}

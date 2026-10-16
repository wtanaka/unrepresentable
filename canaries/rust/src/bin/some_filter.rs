//! Canary.
use std::env::args;

#[rustfmt::skip]
fn main() {
    let flag = args().count() > 3;
    let value: Option<u8> = Some(1).filter(|_| flag); //~ clippy::some_filter
    assert!(value.is_none(), "none");
}

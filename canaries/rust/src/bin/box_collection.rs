//! Canary.
use std::process::exit;

#[rustfmt::skip]
/// Canary item.
fn make() -> Box<Vec<u8>> { //~ clippy::box_collection clippy::unnecessary_box_returns
    Box::default()
}

#[rustfmt::skip]
fn main() {
    exit(i32::from(make().is_empty()));
}

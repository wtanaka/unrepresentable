//! Canary.
#[rustfmt::skip]
/// Canary item.
fn double(value: Option<u8>) -> Option<u8> { //~ clippy::single_option_map
    value.map(|inner| inner.saturating_add(1))
}

#[rustfmt::skip]
fn main() {
    assert!(double(None).is_none(), "none");
}

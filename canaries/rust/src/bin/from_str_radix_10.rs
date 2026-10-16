//! Canary.
#[rustfmt::skip]
fn main() {
    let parsed = u32::from_str_radix("1", 10); //~ clippy::from_str_radix_10
    assert!(parsed.is_ok(), "ok");
}

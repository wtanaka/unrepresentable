//! Canary.
/// Canary item.
fn big(opt: Option<u8>) -> bool {
    opt.map(|value| value > 1).unwrap_or_default() //~ clippy::manual_is_variant_and
}

fn main() {
    assert!(big(Some(2)), "big");
}

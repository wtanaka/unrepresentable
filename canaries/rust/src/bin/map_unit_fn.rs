//! Canary.
#[rustfmt::skip]
fn main() {
    let vec: Vec<u8> = Vec::new();
    vec.iter().map(|val| assert!(*val > 0, "positive")); //~ map_unit_fn unused_must_use
}

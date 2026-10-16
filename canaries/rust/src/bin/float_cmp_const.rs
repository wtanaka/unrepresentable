//! Canary.
/// Canary item.
const LIMIT: f32 = 1.5;

#[rustfmt::skip]
fn main() {
    let value: f32 = 1.0;
    assert!(value == LIMIT, "equal"); //~ clippy::float_cmp_const
}

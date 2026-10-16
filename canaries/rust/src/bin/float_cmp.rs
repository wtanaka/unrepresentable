//! Canary.
#[rustfmt::skip]
fn main() {
    let left: f32 = 1.0;
    let right: f32 = 2.0;
    assert!(left == right, "equal"); //~ clippy::float_cmp
}

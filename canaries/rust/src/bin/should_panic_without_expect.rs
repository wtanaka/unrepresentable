//! Canary.
#[rustfmt::skip]
#[should_panic] //~ clippy::should_panic_without_expect
/// Canary item.
fn fails(value: u8) {
    assert!(value > 1, "boom");
}

#[rustfmt::skip]
fn main() {
    fails(2);
}

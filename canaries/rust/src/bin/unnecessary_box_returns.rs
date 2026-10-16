//! Canary.
#[rustfmt::skip]
/// Canary item.
fn make() -> Box<u8> { //~ clippy::unnecessary_box_returns
    Box::new(1)
}

#[rustfmt::skip]
fn main() {
    assert!(*make() > 0, "positive");
}

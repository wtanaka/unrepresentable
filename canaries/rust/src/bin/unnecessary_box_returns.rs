//! Canary.
#[rustfmt::skip]
/// Canary item.
fn make() -> Box<u8> { //~ clippy::unnecessary_box_returns
    Box::new(1)
}

#[rustfmt::skip]
/// Canary item: the boxed type is above 128 bytes and below 512.
fn make_large() -> Box<[u8; 256]> { //~ clippy::unnecessary_box_returns
    Box::new([1; 256])
}

#[rustfmt::skip]
fn main() {
    assert!(*make() > 0, "positive");
    assert!(make_large().len() == 256, "positive");
}

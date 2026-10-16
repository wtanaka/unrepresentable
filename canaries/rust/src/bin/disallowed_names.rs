//! Canary.
#[rustfmt::skip]
fn main() {
    let foo: u8 = 1; //~ clippy::disallowed_names
    assert!(foo > 0, "positive");
}

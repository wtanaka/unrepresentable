//! Canary.
#[rustfmt::skip]
/// Canary item.
enum Void {}

#[rustfmt::skip]
/// Canary item.
const fn never(value: &Void) -> u8 {
    match *value {} //~ clippy::uninhabited_references
}

#[rustfmt::skip]
fn main() {
    let call: fn(&Void) -> u8 = never;
    assert!(size_of_val(&call) > 0, "fn");
}

//! Canary.
/// Canary item.
const fn pass(val: u8) -> u8 {
    let res = val;
    res //~ clippy::let_and_return
}

fn main() {
    assert_eq!(pass(1), 1, "same");
}

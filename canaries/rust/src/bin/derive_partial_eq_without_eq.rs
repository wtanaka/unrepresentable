//! Canary.
#[derive(Clone, Copy, Debug, PartialEq)] //~ clippy::derive_partial_eq_without_eq
/// Canary item.
pub struct Wrapper(u8);

fn main() {
    let left = Wrapper(1);
    let right = Wrapper(2);
    assert!(left == right, "equal");
}

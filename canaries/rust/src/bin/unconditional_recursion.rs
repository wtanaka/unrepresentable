//! Canary.
/// Canary item.
struct Thing;

#[rustfmt::skip]
impl PartialEq for Thing {
    fn eq(&self, other: &Self) -> bool { //~ clippy::unconditional_recursion
        self == other
    }
}

#[rustfmt::skip]
/// Canary item.
fn recurse() { //~ unconditional_recursion
    recurse();
}

fn main() {
    recurse();
    let first = Thing;
    let second = Thing;
    assert!(first == second, "same");
}

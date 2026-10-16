//! Canary.
#[derive(Hash)] //~ clippy::derived_hash_with_manual_eq
/// Canary item.
struct Wrapper(u8);

impl PartialEq for Wrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Wrapper {}

fn main() {
    let left = Wrapper(1);
    let right = Wrapper(2);
    assert!(left == right, "equal");
}

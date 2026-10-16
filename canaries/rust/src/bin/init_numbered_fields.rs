//! Canary.
/// Canary item.
struct Pair(u8, u8);

fn main() {
    let pair = Pair { 0: 1, 1: 2 }; //~ clippy::init_numbered_fields
    assert_eq!(pair.0, pair.1, "same");
}

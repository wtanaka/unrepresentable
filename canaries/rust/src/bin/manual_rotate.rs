//! Canary.
/// Canary item.
const fn rot(value: u32) -> u32 {
    (value << 3) | (value >> 29) //~ clippy::manual_rotate
}

fn main() {
    assert_eq!(rot(1), 8, "rot");
}

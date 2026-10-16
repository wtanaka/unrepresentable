//! Canary.
/// Canary item.
const fn make() -> bool {
    true
}
fn main() {
    let _v: [bool; 0] = [make(); 0]; //~ clippy::zero_repeat_side_effects
}

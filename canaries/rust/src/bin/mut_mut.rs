//! Canary.
/// Canary item.
const fn takes(_val: &mut &mut u8) {} //~ clippy::mut_mut dead_code
fn main() {}

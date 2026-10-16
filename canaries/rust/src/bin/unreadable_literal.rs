//! Canary.
/// Canary item.
const VALUE: u32 = 1234567; //~ clippy::unreadable_literal
fn main() {
    let _v = VALUE.to_string();
}

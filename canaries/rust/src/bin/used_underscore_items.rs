//! Canary.
/// Canary item.
const fn _helper() -> bool {
    true
}
fn main() {
    let _v = _helper(); //~ clippy::used_underscore_items
}

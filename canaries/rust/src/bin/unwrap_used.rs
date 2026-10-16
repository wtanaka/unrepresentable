//! Canary.
/// Canary item.
const fn make() -> Option<i32> {
    None
}
fn main() {
    let _v = make().unwrap(); //~ clippy::unwrap_used
}

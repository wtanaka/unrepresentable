//! Canary.
/// Canary item.
const fn make() -> Option<i32> { //~ clippy::unnecessary_wraps
    Some(1)
}
fn main() {
    let _v = make();
}

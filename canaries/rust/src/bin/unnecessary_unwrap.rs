//! Canary.
/// Canary item.
const fn make() -> Option<i32> {
    None
}
fn main() {
    let src = make();
    if src.is_some() {
        let _v = src.unwrap(); //~ clippy::unnecessary_unwrap clippy::unwrap_used
    }
}

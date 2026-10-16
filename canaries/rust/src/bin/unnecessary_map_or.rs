//! Canary.
/// Canary item.
const fn make() -> Option<i32> {
    None
}
fn main() {
    let _v = make().map_or(false, i32::is_positive); //~ clippy::unnecessary_map_or
}

//! Canary.
/// Canary item.
const fn make() -> u8 {
    1
}
fn main() {
    // SAFETY: nothing unsafe here
    let _v = make(); //~ clippy::unnecessary_safety_comment
}

//! Canary.
#[non_exhaustive]
/// Canary item.
struct Holder {
    /// Canary item.
    count: i32,
}
/// Canary item.
const fn take(value: &Holder) -> i32 {
    value.count
}
fn main() {
    let mut src = Holder { count: 1 };
    let _v = take(&mut src); //~ clippy::unnecessary_mut_passed
}

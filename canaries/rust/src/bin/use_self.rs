//! Canary.
/// Canary item.
struct Holder {
    /// Canary item.
    count: i32,
}
impl Holder {
    /// Canary item.
    const fn new() -> Holder { //~ clippy::use_self
        Self { count: 1 }
    }
}
fn main() {
    let _v = Holder::new().count;
}

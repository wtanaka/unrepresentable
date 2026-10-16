//! Canary.
/// Canary item.
struct Holder;
impl Holder {
    /// Canary item.
    const fn work(&self) -> i32 { //~ clippy::unused_self
        1
    }
}
fn main() {
    let holder = Holder;
    let _v = holder.work();
}

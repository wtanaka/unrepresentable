//! Canary.
/// Canary item.
struct Holder {
    /// Canary item.
    count: i32,
}
impl Holder {
    /// Canary item.
    fn name(&self) -> &str { //~ clippy::unnecessary_literal_bound
        let _owned = self.count.to_string();
        "name"
    }
}
fn main() {
    let holder = Holder { count: 1 };
    let _v = holder.name();
}

//! Canary.
/// Canary item.
struct Holder {
    /// Canary item.
    name: String,
}
impl Holder {
    /// Canary item.
    fn into_name(&self) -> String { //~ clippy::wrong_self_convention
        self.name.clone()
    }
}
fn main() {
    let holder = Holder {
        name: String::new(),
    };
    let _v = holder.into_name();
}

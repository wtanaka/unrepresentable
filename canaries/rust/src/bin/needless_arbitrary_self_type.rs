//! Canary.
/// Canary item.
struct Thing;
impl Thing {
    /// Canary item.
    const fn consume(self: Self) {} //~ clippy::needless_arbitrary_self_type clippy::unused_self
}
fn main() {
    Thing.consume();
}

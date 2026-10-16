//! Canary.
/// Canary item.
pub(self) const fn func() {} //~ clippy::needless_pub_self
fn main() {
    func();
}

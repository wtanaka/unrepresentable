//! Canary.
/// Canary item.
struct Holder {
    /// Canary item.
    count: i32,
}
fn main() {
    let src = Holder { count: 1 };
    let other = Holder { ..src }; //~ clippy::unnecessary_struct_initialization
    let _v = other.count.to_string();
}

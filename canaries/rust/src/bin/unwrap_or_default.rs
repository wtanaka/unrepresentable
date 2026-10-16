//! Canary.
/// Canary item.
const fn make() -> Option<String> {
    None
}
fn main() {
    let _v = make().unwrap_or_else(String::new); //~ clippy::unwrap_or_default
}

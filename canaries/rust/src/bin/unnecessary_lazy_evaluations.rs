//! Canary.
/// Canary item.
const fn make() -> Option<bool> {
    None
}
fn main() {
    let _v = make().unwrap_or_else(|| true); //~ clippy::unnecessary_lazy_evaluations
}

//! Canary.
/// Canary item.
const fn make() -> Option<bool> {
    None
}
fn main() {
    if let Some(true) | Some(false) = make() { //~ clippy::unnested_or_patterns
        let _v = make();
    }
}

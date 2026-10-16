//! Canary.
/// Canary item.
struct HTTP; //~ clippy::upper_case_acronyms
/// Canary item.
const fn make() -> HTTP {
    HTTP
}
fn main() {
    let _v = make();
}

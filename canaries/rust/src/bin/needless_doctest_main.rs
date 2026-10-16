//! Canary.
/// ```
/// fn main() {} //~ clippy::needless_doctest_main
/// ```
const fn documented() {}
fn main() {
    documented();
}

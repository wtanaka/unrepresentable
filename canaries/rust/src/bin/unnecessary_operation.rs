//! Canary.
/// Canary item.
struct Wrapper(u8);
/// Canary item.
const fn make() -> u8 {
    1
}
fn main() {
    Wrapper(make()); //~ clippy::unnecessary_operation unused_results
    let _v = Wrapper(make()).0;
}

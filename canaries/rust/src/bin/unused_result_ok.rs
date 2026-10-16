//! Canary.
/// Canary item.
const fn make() -> Result<i32, String> {
    Err(String::new())
}
fn main() {
    make().ok(); //~ clippy::unused_result_ok unused_results
}

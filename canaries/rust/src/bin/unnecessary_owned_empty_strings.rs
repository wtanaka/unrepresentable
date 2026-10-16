//! Canary.
/// Canary item.
const fn take(value: &str) -> bool {
    value.is_empty()
}
fn main() {
    let _v = take(&String::new()); //~ clippy::unnecessary_owned_empty_strings
}

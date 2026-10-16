//! Canary.
/// Canary item.
const fn take(value: &str) -> bool {
    value.is_empty()
}
fn main() {
    let src = "a";
    let _v = take(&src.to_string()); //~ clippy::unnecessary_to_owned clippy::str_to_string
}

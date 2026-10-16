//! Canary.
/// Canary item.
const fn take(value: &str) -> bool {
    value.is_empty()
}
fn main() {
    let src = "a";
    let _v = take(src.as_ref()); //~ clippy::useless_asref
}

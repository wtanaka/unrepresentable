//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    sink("abc".replace("ab", "ab")); //~ clippy::no_effect_replace
}

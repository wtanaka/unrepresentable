//! Canary.
/// Canary item.
macro_rules! mac {
    ($($token:tt)*) => {};
}

fn main() {
    mac!(#"foo"#); //~ rust_2024_guarded_string_incompatible_syntax
}

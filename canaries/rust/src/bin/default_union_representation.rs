//! Canary.
use std::process::exit;

#[rustfmt::skip]
/// Canary item.
union Pun { //~ clippy::default_union_representation
    /// Canary item.
    first: u8,
    /// Canary item.
    second: u32, //~ dead_code
}

#[rustfmt::skip]
fn main() {
    let pun = Pun { first: 1 };
    exit(i32::from(size_of_val(&pun) == 2));
}

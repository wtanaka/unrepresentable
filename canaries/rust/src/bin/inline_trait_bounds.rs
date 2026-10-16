//! Canary.
#[rustfmt::skip]
/// Canary item.
const fn same<Val: Copy>(val: Val) -> Val { //~ clippy::inline_trait_bounds
    val
}

fn main() {
    let num: u8 = 1;
    assert_eq!(same(num), 1, "same");
}

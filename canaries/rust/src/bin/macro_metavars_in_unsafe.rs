//! Canary.
/// Canary item.
macro_rules! wrap {
    ($val:expr_2021) => {
        unsafe { $val } //~ clippy::macro_metavars_in_unsafe unsafe_code clippy::undocumented_unsafe_blocks
    };
}

fn main() {
    let one: u8 = 1;
    let num: u8 = wrap!(one.unchecked_add(1));
    assert_eq!(num, 2, "two");
}

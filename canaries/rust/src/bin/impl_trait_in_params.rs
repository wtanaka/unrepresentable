//! Canary.
use std::process::exit;

#[inline]
#[rustfmt::skip]
/// Canary item.
pub fn conv(val: impl Into<i32>) -> i32 { //~ clippy::impl_trait_in_params
    val.into()
}

fn main() {
    let small: i8 = 0;
    exit(conv(small));
}

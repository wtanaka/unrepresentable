//! Canary.
#[rustfmt::skip]
#[inline]
/// Canary item.
pub const fn unused() {} //~ dead_code_pub_in_binary

fn main() {}

//! Canary.
#[no_mangle] //~ unsafe_code unsafe_attr_outside_unsafe
/// Canary item.
fn func() {} //~ clippy::no_mangle_with_rust_abi clippy::missing_const_for_fn
fn main() {}

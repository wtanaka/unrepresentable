//! Canary.
#[no_mangle] //~ unsafe_attr_outside_unsafe unsafe_code
/// Canary item.
pub const extern "C" fn function() {}

fn main() {}

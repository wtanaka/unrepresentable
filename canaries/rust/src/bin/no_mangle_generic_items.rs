//! Canary.
#[no_mangle] //~ unsafe_attr_outside_unsafe unsafe_code
/// Canary item.
pub const extern "C" fn generic<Param>() {} //~ no_mangle_generic_items

fn main() {
    generic::<u8>();
}

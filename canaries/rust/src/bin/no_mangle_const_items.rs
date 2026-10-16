//! Canary.
use core::hint::black_box;

#[no_mangle] //~ unsafe_attr_outside_unsafe unsafe_code
/// Canary item.
pub const FOO: u8 = 1; //~ no_mangle_const_items

fn main() {
    let _: u8 = black_box(FOO);
}

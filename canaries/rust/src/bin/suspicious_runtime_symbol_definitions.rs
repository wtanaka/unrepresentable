//! Canary.
#[rustfmt::skip]
#[no_mangle] //~ unsafe_attr_outside_unsafe unsafe_code
/// Canary item.
pub const extern "C" fn memcpy(dest: *mut u8, _src: *const u8, _count: usize) -> *mut u8 { //~ suspicious_runtime_symbol_definitions
    dest
}

#[rustfmt::skip]
fn main() {}

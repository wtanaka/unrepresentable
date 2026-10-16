//! Canary.
use std::process::exit;

#[rustfmt::skip]
/// Canary item.
const fn shadow<u32>(val: u32) -> u32 { //~ clippy::builtin_type_shadow non_camel_case_types
    val
}

#[rustfmt::skip]
fn main() {
    exit(i32::from(shadow(true)));
}

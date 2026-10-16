//! Canary.
#[rustfmt::skip]
/// Canary item.
fn target() {} //~ clippy::missing_const_for_fn

#[rustfmt::skip]
fn main() {
    let address = target as u64; //~ clippy::fn_to_numeric_cast clippy::fn_to_numeric_cast_any clippy::as_conversions function_casts_as_integer
    assert!(address > 0, "nonzero");
}

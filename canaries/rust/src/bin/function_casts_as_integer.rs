//! Canary.
/// Canary item.
const fn fun() {}

/// Canary item.
fn sink<Ty>(_val: Ty) {}

#[rustfmt::skip]
fn main() {
    sink(fun as usize); //~ function_casts_as_integer clippy::as_conversions clippy::fn_to_numeric_cast_any
}

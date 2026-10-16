//! Canary.
#[rustfmt::skip]
/// Canary item.
pub fn fails() -> Result<u8, ()> { //~ clippy::result_unit_err clippy::missing_const_for_fn clippy::unnecessary_wraps clippy::missing_inline_in_public_items clippy::missing_errors_doc
    Ok(1)
}

#[rustfmt::skip]
fn main() {
    sink(fails());
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

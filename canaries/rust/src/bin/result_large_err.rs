//! Canary.
#[rustfmt::skip]
/// Canary item.
fn big() -> Result<(), [u8; 256]> { //~ clippy::result_large_err clippy::missing_const_for_fn clippy::unnecessary_wraps
    Ok(())
}

#[rustfmt::skip]
fn main() {
    sink(big());
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

//! Canary.
#[rustfmt::skip]
/// Canary item.
fn fail() -> Result<u8, u8> {
    Err(1)?; //~ clippy::try_err
    Ok(1)
}

#[rustfmt::skip]
fn main() {
    assert!(fail().is_err(), "error");
}

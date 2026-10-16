//! Canary.
/// Canary item.
const fn fallible(val: u8) -> Result<u8, u8> {
    if val > 5 {
        Ok(val)
    } else {
        Err(val)
    }
}

fn main() {
    let _ = fallible(1); //~ clippy::let_underscore_must_use clippy::let_underscore_untyped
}

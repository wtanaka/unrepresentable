//! Canary.
#[rustfmt::skip]
/// Canary item.
fn good(res: Result<u8, u8>) -> Option<u8> { //~ clippy::missing_const_for_fn
    match res { //~ clippy::manual_ok_err
        Ok(value) => Some(value),
        Err(_) => None,
    }
}

fn main() {
    assert_eq!(good(Ok(1)), Some(1), "good");
}

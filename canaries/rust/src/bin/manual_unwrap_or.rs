//! Canary.
#[rustfmt::skip]
/// Canary item.
fn pick(opt: Option<u8>) -> u8 { //~ clippy::missing_const_for_fn
    match opt { //~ clippy::manual_unwrap_or
        Some(value) => value,
        None => 5,
    }
}

fn main() {
    assert_eq!(pick(None), 5, "pick");
}

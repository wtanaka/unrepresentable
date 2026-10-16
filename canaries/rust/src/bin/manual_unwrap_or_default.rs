//! Canary.
#[rustfmt::skip]
/// Canary item.
fn pick(opt: Option<u8>) -> u8 { //~ clippy::missing_const_for_fn
    match opt { //~ clippy::manual_unwrap_or_default
        Some(value) => value,
        None => 0,
    }
}

fn main() {
    assert_eq!(pick(None), 0, "pick");
}

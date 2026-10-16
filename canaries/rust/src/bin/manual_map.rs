//! Canary.
#[rustfmt::skip]
/// Canary item.
fn bump(opt: Option<u8>) -> Option<u8> {
    match opt { //~ clippy::manual_map
        Some(value) => Some(value.min(9)),
        None => None,
    }
}

fn main() {
    assert_eq!(bump(Some(1)), Some(1), "bump");
}

//! Canary.
#[rustfmt::skip]
/// Canary item.
fn keep(opt: Option<u8>) -> Option<u8> { //~ clippy::missing_const_for_fn
    match opt { //~ clippy::manual_filter
        Some(value) => {
            if value > 1 { //~ clippy::if_then_some_else_none
                Some(value)
            } else {
                None
            }
        }
        None => None,
    }
}

fn main() {
    assert_eq!(keep(Some(2)), Some(2), "keep");
}

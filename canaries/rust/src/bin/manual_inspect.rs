//! Canary.
#[rustfmt::skip]
/// Canary item.
fn show(opt: Option<u8>) -> Option<u8> { //~ clippy::single_option_map
    opt.map(|value| { //~ clippy::manual_inspect
        check(value);
        value
    })
}

/// Canary item.
fn check(value: u8) {
    assert!(value < 200, "small");
}

fn main() {
    assert_eq!(show(Some(1)), Some(1), "show");
}

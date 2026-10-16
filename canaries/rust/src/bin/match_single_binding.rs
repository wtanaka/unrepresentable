//! Canary.
#[rustfmt::skip]
/// Canary item.
fn pick(value: u8) -> u8 {
    match value { //~ clippy::match_single_binding
        other => other.min(9),
    }
}

fn main() {
    assert_eq!(pick(1), 1, "pick");
}

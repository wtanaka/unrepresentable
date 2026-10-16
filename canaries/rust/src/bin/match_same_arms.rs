//! Canary.
#[rustfmt::skip]
/// Canary item.
fn pick(value: u8) -> u8 { //~ clippy::missing_const_for_fn
    match value {
        1 => 5, //~ clippy::match_same_arms
        2 => 5,
        _ => 6,
    }
}

fn main() {
    assert_eq!(pick(1), 5, "pick");
}

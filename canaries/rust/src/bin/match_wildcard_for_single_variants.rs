//! Canary.
/// Canary item.
enum Color {
    /// Canary item.
    Green,
    /// Canary item.
    Red,
}

#[rustfmt::skip]
/// Canary item.
fn pick(color: &Color) -> u8 { //~ clippy::missing_const_for_fn
    match color {
        Color::Red => 1,
        _ => 2, //~ clippy::match_wildcard_for_single_variants
    }
}

fn main() {
    assert_eq!(pick(&Color::Green), 2, "pick");
    assert_eq!(pick(&Color::Red), 1, "pick");
}

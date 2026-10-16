//! Canary.
#[rustfmt::skip]
/// Canary item.
enum Fruit { //~ clippy::enum_variant_names
    /// Canary item.
    AppleGreen,
    /// Canary item.
    AppleRed,
    /// Canary item.
    AppleYellow,
}

fn main() {
    let all = [Fruit::AppleRed, Fruit::AppleGreen, Fruit::AppleYellow];
    assert!(all.len() == 3, "three");
}

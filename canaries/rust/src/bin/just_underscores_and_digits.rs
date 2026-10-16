//! Canary.
/// Canary item.
const fn skip(_1: u8) { //~ clippy::just_underscores_and_digits
}

fn main() {
    skip(1);
}

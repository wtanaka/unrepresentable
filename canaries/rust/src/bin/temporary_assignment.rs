//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Pair {
    /// Canary item.
    left: u8,
}

#[rustfmt::skip]
fn main() {
    Pair { left: 1 }.left = 2; //~ clippy::temporary_assignment
}

//! Canary.
#[rustfmt::skip]
/// Canary item.
const fn truth() -> bool {
    true
}

#[rustfmt::skip]
fn main() {
    let left = truth();
    left && truth(); //~ clippy::short_circuit_statement unused_must_use
}

//! Canary.
fn main() {
    let _v: Vec<bool> = Vec::with_capacity(0); //~ clippy::with_capacity_zero
}

//! Canary.
fn main() {
    let val: u8 = 1;
    (val, val); //~ clippy::no_effect unused_results
}

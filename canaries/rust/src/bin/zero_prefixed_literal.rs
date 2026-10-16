//! Canary.
fn main() {
    let value: i32 = 0123; //~ clippy::zero_prefixed_literal
    let _v = value.to_string();
}

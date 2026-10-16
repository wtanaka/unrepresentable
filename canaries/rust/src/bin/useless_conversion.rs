//! Canary.
fn main() {
    let src = String::new();
    let _v = String::from(src); //~ clippy::useless_conversion
}

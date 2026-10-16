//! Canary.
fn main() {
    let value: i32 = 1;
    let _v = format!("{value}",); //~ clippy::unnecessary_trailing_comma
}

//! Canary.
fn main() {
    let src: [i32; 2] = [1, 2];
    let _v: bool = src.first().is_some(); //~ clippy::unnecessary_first_then_check
}

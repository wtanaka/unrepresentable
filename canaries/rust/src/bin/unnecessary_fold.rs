//! Canary.
fn main() {
    let src: [bool; 2] = [true, false];
    let _v: bool = src.iter().fold(false, |acc, item| acc || *item); //~ clippy::unnecessary_fold
}

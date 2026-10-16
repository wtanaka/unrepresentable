//! Canary.
fn main() {
    let start: u8 = 0;
    let num = (start..).count(); //~ clippy::infinite_iter
    assert_eq!(num, 0, "zero");
}

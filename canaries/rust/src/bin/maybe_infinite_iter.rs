//! Canary.
fn main() {
    let start: u8 = 0;
    let total = (start..).take_while(|num| *num < 5).count(); //~ clippy::maybe_infinite_iter
    assert_eq!(total, 5, "total");
}

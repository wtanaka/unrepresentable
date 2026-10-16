//! Canary.
#[rustfmt::skip]
fn main() {
    let count = " a b ".trim().split_whitespace().count(); //~ clippy::trim_split_whitespace
    assert_eq!(count, 2, "two");
}

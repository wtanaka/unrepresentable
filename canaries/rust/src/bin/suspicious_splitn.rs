//! Canary.
#[rustfmt::skip]
fn main() {
    let count = "a,b".splitn(1, ',').count(); //~ clippy::suspicious_splitn
    assert_eq!(count, 1, "one");
}

//! Canary.
#[rustfmt::skip]
fn main() {
    let text = "a\nb";
    let count = text.trim().split('\n').count(); //~ clippy::str_split_at_newline
    assert_eq!(count, 2, "two");
}

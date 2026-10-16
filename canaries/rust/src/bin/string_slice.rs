//! Canary.
#[rustfmt::skip]
fn main() {
    let text = "hello";
    assert_eq!(&text[1..3], "el", "slice"); //~ clippy::string_slice
}

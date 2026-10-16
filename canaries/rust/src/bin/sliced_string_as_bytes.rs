//! Canary.
#[rustfmt::skip]
fn main() {
    let text = "hello";
    let bytes = text[1..3].as_bytes(); //~ clippy::sliced_string_as_bytes clippy::string_slice
    assert_eq!(bytes.len(), 2, "len");
}

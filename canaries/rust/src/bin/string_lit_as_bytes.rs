//! Canary.
#[rustfmt::skip]
fn main() {
    let bytes = "hello".as_bytes(); //~ clippy::string_lit_as_bytes
    assert_eq!(bytes.len(), 5, "len");
}

//! Canary.
#[rustfmt::skip]
fn main() {
    let value: Option<u8> = Some(2);
    if let Some(2) = value { //~ clippy::equatable_if_let
        assert!(value.is_some(), "some");
    }
}

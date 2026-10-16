//! Canary.
fn main() {
    let num = Option::<u8>::None.into_iter().count(); //~ clippy::iter_on_empty_collections
    assert_eq!(num, 0, "empty");
}

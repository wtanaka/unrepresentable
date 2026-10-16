//! Canary.
fn main() {
    let data: Vec<Option<u8>> = Vec::new();
    let num = data.into_iter().filter(Option::is_some).count(); //~ clippy::iter_filter_is_some
    assert_eq!(num, 0, "empty");
}

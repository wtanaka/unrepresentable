//! Canary.
fn main() {
    let data: Vec<Result<u8, u8>> = Vec::new();
    let num = data.into_iter().filter(Result::is_ok).count(); //~ clippy::iter_filter_is_ok
    assert_eq!(num, 0, "empty");
}

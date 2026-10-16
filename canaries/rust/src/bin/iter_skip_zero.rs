//! Canary.
fn main() {
    let data: Vec<u8> = Vec::new();
    let num = data.iter().skip(0).count(); //~ clippy::iter_skip_zero
    assert_eq!(num, 0, "empty");
}

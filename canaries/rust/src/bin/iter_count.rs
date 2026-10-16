//! Canary.
fn main() {
    let data: Vec<u8> = Vec::new();
    let num = data.iter().count(); //~ clippy::iter_count
    assert_eq!(num, 0, "empty");
}

//! Canary.
#[rustfmt::skip]
fn main() {
    let data: [u8; 2] = [1, 2];
    let count = data.iter().map(|each| each.checked_add(1)).count(); //~ clippy::suspicious_map
    assert_eq!(count, 2, "two");
}

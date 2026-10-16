//! Canary.
fn main() {
    let count: i32 = 3; //~ clippy::needless_type_cast
    let doubled = count as i64; //~ clippy::as_conversions clippy::cast_lossless
    let tripled = count as i64; //~ clippy::as_conversions clippy::cast_lossless
    assert_eq!(doubled, tripled, "same");
}

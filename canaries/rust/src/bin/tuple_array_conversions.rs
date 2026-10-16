//! Canary.
#[rustfmt::skip]
fn main() {
    let pair: (u8, u8) = (1, 2);
    let array = [pair.0, pair.1]; //~ clippy::tuple_array_conversions
    assert_eq!(array.len(), 2, "two");
}

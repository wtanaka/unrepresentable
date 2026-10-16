//! Canary.
#[rustfmt::skip]
/// Canary item.
fn many(one: u8, two: u8, three: u8, four: u8, five: u8, six: u8, seven: u8, eight: u8) -> u8 { //~ clippy::too_many_arguments
    one.max(two)
        .max(three)
        .max(four)
        .max(five)
        .max(six)
        .max(seven)
        .max(eight)
}

#[rustfmt::skip]
fn main() {
    assert!(many(1, 2, 3, 4, 5, 6, 7, 8) > 0, "positive");
}

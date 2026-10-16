//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let first: i32 = 1;
    let second: i32 = 2;
    let third: i32 = 3;
    let fourth: i32 = 4;
    let list = [
        first,
        second //~ clippy::possible_missing_comma
        -third,
        fourth,
    ];
    sink(list);
}

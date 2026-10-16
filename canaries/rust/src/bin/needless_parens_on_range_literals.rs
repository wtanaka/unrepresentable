//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let range = ('a')..=('z'); //~ clippy::needless_parens_on_range_literals
    sink(range);
}

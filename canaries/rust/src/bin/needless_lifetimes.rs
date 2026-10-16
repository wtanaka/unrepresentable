//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Thing;

#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
/// Canary item.
const fn pick<'a>(val: &'a Thing) -> &'a Thing { //~ clippy::single_char_lifetime_names clippy::needless_lifetimes
    val
}

#[rustfmt::skip]
fn main() {
    sink(pick(&Thing));
}

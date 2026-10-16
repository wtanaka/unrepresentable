//! Canary.
use core::num::NonZeroU8;
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let small = NonZeroU8::MIN;
    let big = u16::from(small.get()); //~ clippy::non_zero_suggestions
    sink(big);
}

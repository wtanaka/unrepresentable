//! Canary.
use core::hint::black_box;
#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    let opt: Option<u8> = black_box(None);
    let same = match opt { //~ clippy::needless_match clippy::manual_map
        Some(inner) => Some(inner),
        None => None,
    };
    sink(same);
}

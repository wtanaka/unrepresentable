//! Canary.
#[rustfmt::skip]
fn main() {
    let items: Vec<u8> = Vec::new();
    for pair in items.iter().zip(0..items.len()) { //~ clippy::range_zip_with_len
        sink(pair);
    }
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

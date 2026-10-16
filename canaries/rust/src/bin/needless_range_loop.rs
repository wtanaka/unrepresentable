//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    let arr: [u8; 2] = [1, 2];
    for idx in 0..arr.len() { //~ clippy::needless_range_loop
        sink(arr[idx]); //~ clippy::indexing_slicing
    }
}

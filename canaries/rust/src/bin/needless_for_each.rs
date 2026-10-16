//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    let arr: [u8; 2] = [1, 2];
    arr.iter().for_each(|val| { //~ clippy::needless_for_each
        sink(val);
    });
}

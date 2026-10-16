//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    let arr: [u8; 3] = [0, 1, 2];
    for val in arr {
        if val > 0 {
            sink(val);
        } else { //~ clippy::needless_continue
            continue;
        }
    }
}

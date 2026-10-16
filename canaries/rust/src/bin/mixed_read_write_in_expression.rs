//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    let mut num: u32 = 0;
    let total = num | { //~ clippy::mixed_read_write_in_expression
        num = 1;
        num
    };
    sink(total);
}

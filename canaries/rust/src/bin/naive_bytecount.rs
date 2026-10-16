//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

/// Canary item.
fn count(hay: &[u8], needle: u8) -> usize {
    hay.iter().filter(|&&byte| byte == needle).count() //~ clippy::naive_bytecount
}
fn main() {
    sink(count(&[], 0));
}

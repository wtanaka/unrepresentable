//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let arr: [u8; 2] = [1, 2];
    let items = arr.iter().collect::<Vec<_>>(); //~ clippy::needless_collect
    sink(items.len());
}

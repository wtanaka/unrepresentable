//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let arr: [u8; 2] = [1, 2];
    sink(arr[2]); //~ clippy::out_of_bounds_indexing unconditional_panic
}

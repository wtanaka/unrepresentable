//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let opt: Option<u8> = None;
    sink(opt.or(Some(1)).unwrap()); //~ clippy::or_then_unwrap clippy::unwrap_used
}

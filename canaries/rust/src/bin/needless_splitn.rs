//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let text = "a b";
    sink(text.splitn(2, ' ').next()); //~ clippy::needless_splitn
}

//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let num: u32 = 1;
    let &ref bound = &num; //~ clippy::needless_borrowed_reference clippy::ref_patterns
    sink(bound);
}

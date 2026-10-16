//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let flag = true;
    sink(flag && !flag); //~ clippy::overly_complex_bool_expr
}

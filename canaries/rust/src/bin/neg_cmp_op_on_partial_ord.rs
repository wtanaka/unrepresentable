//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let lhs: f64 = 1.0;
    let rhs: f64 = 2.0;
    let res = !(lhs < rhs); //~ clippy::neg_cmp_op_on_partial_ord
    sink(res);
}

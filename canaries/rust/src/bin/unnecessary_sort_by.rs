//! Canary.
fn main() {
    let mut src: Vec<i32> = Vec::new();
    src.sort_by(|left, right| left.cmp(right)); //~ clippy::unnecessary_sort_by clippy::redundant_closure_for_method_calls
}

//! Canary.
fn main() {
    let mut src: Vec<i32> = Vec::new();
    src.resize(0, 5); //~ clippy::vec_resize_to_zero clippy::default_numeric_fallback
    let _v = src.len();
}

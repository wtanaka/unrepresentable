//! Canary.
fn main() {
    let mut src = Vec::new(); //~ clippy::vec_init_then_push
    src.push(true);
    let _v = src.len();
}

//! Canary.
/// Canary item.
fn make() -> Vec<Box<u8>> { //~ clippy::vec_box
    let src = String::new();
    let _v = src.len();
    Vec::new()
}
fn main() {
    let _v = make().len();
}

//! Canary.
/// Canary item.
fn make() -> Vec<Box<u8>> { //~ clippy::vec_box
    let src = String::new();
    let _v = src.len();
    Vec::new()
}
/// Canary item: the boxed type is above 128 bytes and below 16384.
fn make_large() -> Vec<Box<[u8; 1024]>> { //~ clippy::vec_box
    let src = String::new();
    let _v = src.len();
    Vec::new()
}
fn main() {
    let _v = make().len();
    let _w = make_large().len();
}

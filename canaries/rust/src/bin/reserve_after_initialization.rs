//! Canary.
#[rustfmt::skip]
fn main() {
    let mut list: Vec<u8> = Vec::new(); //~ clippy::reserve_after_initialization
    list.reserve(4);
    drop(list);
}

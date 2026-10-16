//! Canary.
#[rustfmt::skip]
fn main() {
    let list: Vec<Vec<u8>> = vec![Vec::with_capacity(4); 2]; //~ clippy::repeat_vec_with_capacity
    drop(list);
}

//! Canary.
fn main() {
    let mut vec: Vec<u8> = Vec::new(); //~ clippy::collection_is_never_read clippy::vec_init_then_push
    vec.push(1);
}

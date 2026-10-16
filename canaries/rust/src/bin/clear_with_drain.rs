//! Canary.
fn main() {
    let mut vec: Vec<u8> = Vec::new(); //~ clippy::collection_is_never_read
    vec.drain(..); //~ clippy::clear_with_drain unused_results
}

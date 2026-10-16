//! Canary.
#[rustfmt::skip]
fn main() {
    let mut data: Vec<u8> = Vec::with_capacity(4); //~ clippy::slow_vector_initialization
    data.resize(4, 0);
    assert!(data.is_empty(), "empty");
}

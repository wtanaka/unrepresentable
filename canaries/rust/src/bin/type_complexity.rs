//! Canary.
#[rustfmt::skip]
fn main() {
    let data: Vec<Vec<Box<(u32, u32, u32, u32)>>> = Vec::new(); //~ clippy::type_complexity
    assert!(data.is_empty(), "empty");
}

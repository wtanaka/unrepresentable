//! Canary.
fn main() {
    let data: Vec<u8> = Vec::new();
    let refd = &data;
    let first = refd.into_iter().next(); //~ clippy::into_iter_on_ref
    assert!(first.is_none(), "empty");
}

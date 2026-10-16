//! Canary.
#[rustfmt::skip]
fn main() {
    let val: u8 = 1;
    if let _bound = val {} //~ irrefutable_let_patterns
}

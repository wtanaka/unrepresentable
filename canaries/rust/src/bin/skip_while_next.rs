//! Canary.
#[rustfmt::skip]
fn main() {
    let data: [u8; 2] = [1, 2];
    let found = data.iter().skip_while(|item| **item < 2).next(); //~ clippy::skip_while_next
    assert!(found.is_some(), "some");
}

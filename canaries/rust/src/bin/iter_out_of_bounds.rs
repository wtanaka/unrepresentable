//! Canary.
fn main() {
    let top: Option<u8> = [1, 2, 3].into_iter().skip(4).max(); //~ clippy::iter_out_of_bounds
    assert!(top.is_none(), "empty");
}

//! Canary.
use core::slice::Iter;

#[rustfmt::skip]
/// Canary item.
fn first(arg: &[u8]) -> Iter<u8> { //~ mismatched_lifetime_syntaxes elided_lifetimes_in_paths
    arg.iter()
}

fn main() {
    let arr: [u8; 1] = [1];
    assert_eq!(first(&arr).len(), 1, "len");
}

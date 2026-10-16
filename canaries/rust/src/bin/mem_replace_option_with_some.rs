//! Canary.
use core::mem;

#[rustfmt::skip]
/// Canary item.
fn put(opt: &mut Option<u8>) -> Option<u8> { //~ clippy::missing_const_for_fn
    mem::replace(opt, Some(1)) //~ clippy::mem_replace_option_with_some
}

fn main() {
    let mut opt = None;
    assert_eq!(put(&mut opt), None, "put");
}

//! Canary.
use core::mem;

#[rustfmt::skip]
/// Canary item.
fn take(opt: &mut Option<u8>) -> Option<u8> { //~ clippy::missing_const_for_fn
    mem::replace(opt, None) //~ clippy::mem_replace_option_with_none
}

fn main() {
    let mut opt = Some(1);
    assert_eq!(take(&mut opt), Some(1), "take");
}

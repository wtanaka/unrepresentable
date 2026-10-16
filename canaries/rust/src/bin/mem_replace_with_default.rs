//! Canary.
use core::mem;

#[rustfmt::skip]
/// Canary item.
fn take(items: &mut Vec<u8>) -> Vec<u8> { //~ clippy::missing_const_for_fn
    mem::replace(items, Vec::new()) //~ clippy::mem_replace_with_default
}

fn main() {
    let mut items: Vec<u8> = Vec::new();
    assert!(take(&mut items).is_empty(), "take");
}

//! Canary.
use core::hint::black_box;

/// Canary item.
type Alias<Elem: Copy> = Vec<Elem>; //~ type_alias_bounds

fn main() {
    let _unused: Alias<u8> = black_box(Vec::new());
}

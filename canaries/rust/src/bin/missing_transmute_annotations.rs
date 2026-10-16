//! Canary.
use core::mem;

/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let src: u32 = 1;
    // SAFETY: u32 and i32 have identical layout.
    sink::<i32>(unsafe { mem::transmute(src) }); //~ clippy::missing_transmute_annotations unsafe_code unnecessary_transmutes
}

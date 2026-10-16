//! Canary.
use core::hint::black_box;

/// Canary item.
struct Wrapper(usize);

impl Wrapper {
    /// Canary item.
    const fn run() {
        const ZERO: Wrapper = Self(0); //~ self_constructor_from_outer_item
        let _: usize = black_box(ZERO.0);
    }
}

fn main() {
    Wrapper::run();
}

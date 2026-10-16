//! Canary.
use core::hint::black_box;

/// Canary item.
enum Choice { //~ clippy::large_enum_variant
    /// Canary item.
    Large([u8; 100]), //~ variant_size_differences
    /// Canary item.
    Small(u8),
}

/// Canary item.
fn size(value: &Choice) -> usize {
    match value {
        Choice::Small(byte) => usize::from(*byte),
        Choice::Large(array) => array.len(),
    }
}

fn main() {
    let _: usize = black_box(size(&Choice::Small(1)));
    let _: usize = black_box(size(&Choice::Large([0; 100])));
}

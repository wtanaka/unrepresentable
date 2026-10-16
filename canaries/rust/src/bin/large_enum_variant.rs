//! Canary.
#[rustfmt::skip]
/// Canary item.
enum Big { //~ clippy::large_enum_variant
    /// Canary item.
    Huge([u8; 1000]), //~ variant_size_differences
    /// Canary item.
    Small(u8),
}

/// Canary item.
fn size(val: &Big) -> usize {
    match val {
        Big::Huge(arr) => arr.len(),
        Big::Small(num) => usize::from(*num),
    }
}

fn main() {
    let huge = Big::Huge([0; 1000]); //~ clippy::large_stack_arrays
    let small = Big::Small(1);
    assert!(size(&huge) > size(&small), "bigger");
}

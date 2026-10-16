//! Canary.
#[rustfmt::skip]
#[repr(packed)]
/// Canary item.
struct Packed { //~ clippy::repr_packed_without_abi
    /// Canary item.
    val: u32,
    /// Canary item.
    other: u8,
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let packed = Packed { val: 1, other: 2 };
    sink(packed.other);
    sink(packed.val);
}

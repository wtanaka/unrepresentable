//! Canary.
/// Canary item.
fn sink<Ty>(_val: Ty) {}

#[rustfmt::skip]
fn main() {
    let arr_a: [u8; 1] = [1];
    let arr_b: [u8; 1] = [2];
    let ptr_a: *const [u8] = &arr_a;
    let ptr_b: *const [u8] = &arr_b;
    sink(ptr_a == ptr_b); //~ ambiguous_wide_pointer_comparisons
}

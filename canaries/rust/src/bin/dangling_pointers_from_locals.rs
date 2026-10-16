//! Canary.
#[rustfmt::skip]
/// Canary item.
fn dangle() -> *const u8 { //~ clippy::missing_const_for_fn
    let val: u8 = 1;
    &val //~ dangling_pointers_from_locals clippy::borrow_as_ptr
}

fn main() {
    let ptr: *const u8 = dangle();
    assert!(!ptr.is_null(), "non-null");
}

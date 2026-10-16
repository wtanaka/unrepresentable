//! Canary.
/// Canary item.
const fn dangling() -> *const u32 {
    align_of::<u32>() as *const u32 //~ clippy::as_conversions clippy::manual_dangling_ptr
}

fn main() {
    assert!(!dangling().is_null(), "null");
}

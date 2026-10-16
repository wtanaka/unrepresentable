//! Canary.
use core::ffi::CStr;

/// Canary item.
fn name() -> &'static CStr {
    CStr::from_bytes_with_nul(b"foo\0").unwrap() //~ clippy::manual_c_str_literals clippy::unwrap_used
}

fn main() {
    assert_eq!(name().to_bytes().len(), 3, "len");
}

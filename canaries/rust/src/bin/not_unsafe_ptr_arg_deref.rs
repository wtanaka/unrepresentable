//! Canary.
#[rustfmt::skip]
/// Canary item.
pub mod outer { //~ clippy::inline_modules
    /// Canary item.
    pub const fn read(ptr: *const u8) -> u8 { //~ clippy::must_use_candidate clippy::missing_inline_in_public_items
        // SAFETY: none.
        unsafe { *ptr } //~ unsafe_code clippy::not_unsafe_ptr_arg_deref
    }
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    sink(outer::read(&1));
}

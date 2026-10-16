//! Canary.
#[rustfmt::skip]
/// Canary item.
pub mod outer { //~ clippy::inline_modules
    /// Canary item.
    pub const fn double(val: u8) -> u8 { //~ clippy::must_use_candidate clippy::missing_inline_in_public_items
        val
    }
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    sink(outer::double(1));
}

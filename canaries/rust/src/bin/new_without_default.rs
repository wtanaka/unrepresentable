//! Canary.
#[rustfmt::skip]
/// Canary item.
pub mod outer { //~ clippy::inline_modules
    /// Canary item.
    pub struct Thing; //~ clippy::exhaustive_structs missing_copy_implementations missing_debug_implementations
    impl Thing {
        #[must_use]
        /// Canary item.
        pub const fn new() -> Self { //~ clippy::new_without_default clippy::missing_inline_in_public_items
            Self
        }
    }
}
#[rustfmt::skip]
fn main() {
    drop(outer::Thing::new());
}

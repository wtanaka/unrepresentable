//! Canary.
/// Canary item.
macro_rules! outer {
    () => {
        #[macro_export]
        /// Canary item.
        macro_rules! inner {
            () => {};
        }
    };
}

outer!();

#[rustfmt::skip]
fn main() {
    crate::inner!(); //~ macro_expanded_macro_exports_accessed_by_absolute_paths
}

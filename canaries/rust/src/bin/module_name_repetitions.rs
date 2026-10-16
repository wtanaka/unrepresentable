//! Canary.
#[rustfmt::skip]
/// Canary item.
pub mod foo { //~ clippy::inline_modules
    /// Canary item.
    pub struct FooBar; //~ clippy::exhaustive_structs clippy::module_name_repetitions missing_copy_implementations missing_debug_implementations
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    sink(foo::FooBar);
}

//! Canary.
#[rustfmt::skip]
/// Canary item.
mod one { //~ clippy::inline_modules
    /// Canary item.
    pub struct Item; //~ dead_code_pub_in_binary clippy::exhaustive_structs missing_copy_implementations missing_debug_implementations
}

#[rustfmt::skip]
/// Canary item.
mod two { //~ clippy::inline_modules
    /// Canary item.
    pub struct Item; //~ dead_code unreachable_pub
}

#[rustfmt::skip]
/// Canary item.
pub mod out { //~ clippy::inline_modules
    pub use super::one::*; //~ clippy::pub_use ambiguous_glob_reexports
    pub use super::two::*; //~ clippy::pub_use unused_imports unreachable_pub
}

fn main() {}

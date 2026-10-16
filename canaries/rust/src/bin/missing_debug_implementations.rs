//! Canary.
#[rustfmt::skip]
/// Canary item.
pub mod out { //~ clippy::inline_modules
    /// Canary item.
    pub struct Item; //~ missing_copy_implementations missing_debug_implementations dead_code_pub_in_binary clippy::exhaustive_structs
}

fn main() {}

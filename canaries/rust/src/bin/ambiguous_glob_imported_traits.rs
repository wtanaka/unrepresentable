//! Canary.
#[rustfmt::skip]
/// Canary item.
mod one { //~ clippy::inline_modules
    /// Canary item.
    pub(crate) trait Tr {
        /// Canary item.
        fn go(&self) {}
    }
    impl Tr for u8 {}
}

#[rustfmt::skip]
/// Canary item.
mod two { //~ clippy::inline_modules
    /// Canary item.
    pub(crate) trait Tr { //~ dead_code
        /// Canary item.
        fn go(&self) {}
    }
    impl Tr for u8 {}
}

#[rustfmt::skip]
use one::*; //~ clippy::wildcard_imports
#[rustfmt::skip]
use two::*; //~ unused_imports

#[rustfmt::skip]
fn main() {
    let num: u8 = 1;
    num.go(); //~ ambiguous_glob_imported_traits
}

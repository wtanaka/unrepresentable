//! Canary.
#[rustfmt::skip]
/// Canary item.
macro_rules! mac { //~ unused_macros
    ($val:expr) => { //~ edition_2024_expr_fragment_specifier
        $other //~ meta_variable_misuse
    };
}

fn main() {}

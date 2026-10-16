//! Canary.
#[rustfmt::skip]
/// Canary item.
mod one { //~ clippy::inline_modules
    unsafe extern "C" { //~ unsafe_code
        pub(crate) fn clash(arg: u8);
    }
}

#[rustfmt::skip]
/// Canary item.
mod two { //~ clippy::inline_modules
    unsafe extern "C" { //~ unsafe_code
        pub(crate) fn clash(arg: u16); //~ clashing_extern_declarations
    }
}

/// Canary item.
fn sink<Ty>(_val: Ty) {}

fn main() {
    sink(one::clash);
    sink(two::clash);
}

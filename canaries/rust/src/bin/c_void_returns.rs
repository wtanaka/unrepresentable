//! Canary.
use core::ffi::c_void;

#[rustfmt::skip]
unsafe extern "C" { //~ unsafe_code
    fn get() -> c_void; //~ c_void_returns
}

/// Canary item.
fn sink<Ty>(_val: Ty) {}

fn main() {
    sink(get);
}

//! Canary.
#[rustfmt::skip]
unsafe extern "C" { //~ unsafe_code
    fn takes(arg: &str); //~ improper_ctypes
}

/// Canary item.
fn sink<Ty>(_val: Ty) {}

fn main() {
    sink(takes);
}

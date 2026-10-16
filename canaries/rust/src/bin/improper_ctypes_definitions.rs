//! Canary.
#[rustfmt::skip]
/// Canary item.
extern "C" fn takes(_arg: &str) {} //~ improper_ctypes_definitions clippy::missing_const_for_fn

/// Canary item.
fn sink<Ty>(_val: Ty) {}

fn main() {
    sink(takes);
}

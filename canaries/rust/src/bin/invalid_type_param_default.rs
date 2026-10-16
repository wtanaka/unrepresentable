//! Canary.
#[rustfmt::skip]
/// Canary item.
fn takes<Ty = u8>(_arg: Ty) {} //~ invalid_type_param_default

fn main() {
    let num: u8 = 1;
    takes(num);
}

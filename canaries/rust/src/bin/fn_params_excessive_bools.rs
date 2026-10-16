//! Canary.
#[rustfmt::skip]
/// Canary item.
fn choose(flag: bool) -> u8 { //~ clippy::fn_params_excessive_bools
    u8::from(flag)
}

fn main() {
    assert!(choose(true) == 1, "one");
}

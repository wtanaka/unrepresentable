//! Canary.
#[rustfmt::skip]
/// Canary item.
fn pick(flag: bool) -> u8 { //~ clippy::fn_params_excessive_bools clippy::missing_const_for_fn
    match flag { //~ clippy::match_bool
        true => 1,
        false => 2,
    }
}

fn main() {
    assert_eq!(pick(true), 1, "pick");
}

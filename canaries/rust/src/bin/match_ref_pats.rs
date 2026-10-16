//! Canary.
#[rustfmt::skip]
/// Canary item.
fn pick(opt: &Option<u8>) -> u8 { //~ clippy::missing_const_for_fn clippy::ref_option clippy::trivially_copy_pass_by_ref
    match opt { //~ clippy::match_ref_pats
        &Some(value) => value,
        &None => 0,
    }
}

fn main() {
    assert_eq!(pick(&None), 0, "pick");
}

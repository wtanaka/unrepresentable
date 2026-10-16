//! Canary.
/// Doc.
#[rustfmt::skip]
pub fn boom(opt: Option<u8>) -> u8 { //~ clippy::missing_const_for_fn clippy::missing_inline_in_public_items clippy::missing_panics_doc clippy::must_use_candidate
    opt.unwrap() //~ clippy::unwrap_used
}

fn main() {
    assert_eq!(boom(Some(1)), 1, "boom");
}

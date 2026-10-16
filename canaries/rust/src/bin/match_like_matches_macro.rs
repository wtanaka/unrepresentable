//! Canary.
#[rustfmt::skip]
/// Canary item.
fn is_one(opt: Option<u8>) -> bool { //~ clippy::missing_const_for_fn
    match opt { Some(1) => true, _ => false } //~ clippy::match_like_matches_macro
}

fn main() {
    assert!(is_one(Some(1)), "one");
}

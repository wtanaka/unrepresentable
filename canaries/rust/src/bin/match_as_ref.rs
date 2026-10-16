//! Canary.
#[rustfmt::skip]
/// Canary item.
fn view(opt: &Option<String>) -> Option<&String> { //~ clippy::missing_const_for_fn clippy::ref_option
    match *opt { //~ clippy::match_as_ref
        Some(ref value) => Some(value), //~ clippy::ref_patterns
        None => None,
    }
}

fn main() {
    assert!(view(&None).is_none(), "view");
}

//! Canary.
#[rustfmt::skip]
/// Canary item.
fn pick(text: &str) -> u8 {
    match text.to_lowercase().as_str() {
        "Abc" => 1, //~ clippy::match_str_case_mismatch
        _ => 2,
    }
}

fn main() {
    assert_eq!(pick("abc"), 2, "pick");
}

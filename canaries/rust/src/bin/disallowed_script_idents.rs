//! Canary.
#[rustfmt::skip]
fn main() {
    let привет: u8 = 1; //~ clippy::disallowed_script_idents non_ascii_idents
    assert!(привет > 0, "positive");
}

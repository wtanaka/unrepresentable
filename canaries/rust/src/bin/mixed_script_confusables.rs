//! Canary.
#[rustfmt::skip]
fn main() {
    let ab: u8 = 1;
    let аb: u8 = 2; //~ confusable_idents mixed_script_confusables non_ascii_idents clippy::disallowed_script_idents
    assert_eq!(ab, аb, "same");
}

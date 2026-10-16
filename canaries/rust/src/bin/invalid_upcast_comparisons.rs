//! Canary.
fn main() {
    let val: u8 = 3;
    #[expect(clippy::as_conversions, clippy::cast_lossless, reason = "canary")]
    let big = (val as u32) > 300; //~ clippy::invalid_upcast_comparisons
    assert!(!big, "never");
    assert_eq!(val, 3, "same");
}

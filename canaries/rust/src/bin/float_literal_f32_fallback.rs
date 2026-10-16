//! Canary.
#[rustfmt::skip]
fn main() {
    let val = 1.0; //~ float_literal_f32_fallback
    let flt: f32 = val.into(); //~ clippy::useless_conversion
    assert_eq!(flt.to_bits(), 1, "bits");
}

//! Canary.
/// Canary item.
fn sink<Ty>(_val: Ty) {}

#[rustfmt::skip]
fn main() {
    let val: f32 = 1.0;
    sink(val < f32::NAN); //~ invalid_nan_comparisons
}

//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Holder<'life, Ty: 'life>(&'life Ty); //~ explicit_outlives_requirements

fn main() {
    let num: u8 = 1;
    assert_eq!(*Holder(&num).0, 1, "value");
}

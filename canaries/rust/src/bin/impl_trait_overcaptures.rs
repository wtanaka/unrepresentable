//! Canary.
#[rustfmt::skip]
/// Canary item.
fn get(arg: &u8) -> impl Sized { //~ impl_trait_overcaptures clippy::trivially_copy_pass_by_ref
    *arg
}

/// Canary item.
fn sink<Ty>(_val: Ty) {}

fn main() {
    let num: u8 = 1;
    sink(get(&num));
}

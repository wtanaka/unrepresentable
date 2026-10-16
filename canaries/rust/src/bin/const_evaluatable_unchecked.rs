//! Canary.
#[rustfmt::skip]
/// Canary item.
fn make<Ty>() -> usize { //~ clippy::missing_const_for_fn
    let arr = [u8::MIN; size_of::<*mut Ty>()]; //~ const_evaluatable_unchecked
    arr.len()
}

fn main() {
    assert_eq!(make::<u8>(), size_of::<usize>(), "len");
}

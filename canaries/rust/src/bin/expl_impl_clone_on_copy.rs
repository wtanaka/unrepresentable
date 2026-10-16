//! Canary.
#[derive(Copy)]
/// Canary item.
struct Wrapper(u8);

#[rustfmt::skip]
impl Clone for Wrapper { //~ clippy::expl_impl_clone_on_copy
    fn clone(&self) -> Self {
        *self
    }
}

fn main() {
    let value = Wrapper(1);
    let copy = value;
    assert!(copy.0 == value.0, "same");
}

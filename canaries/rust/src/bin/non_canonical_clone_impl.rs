//! Canary.
#[rustfmt::skip]
#[derive(Copy)]
/// Canary item.
struct Thing(u8);

#[rustfmt::skip]
impl Clone for Thing { //~ clippy::expl_impl_clone_on_copy
    fn clone(&self) -> Self { //~ clippy::non_canonical_clone_impl
        Self(self.0)
    }
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    sink(Thing(1).clone()); //~ clippy::clone_on_copy
}

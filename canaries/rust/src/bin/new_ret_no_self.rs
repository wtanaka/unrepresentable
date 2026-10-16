//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Thing;

#[rustfmt::skip]
impl Thing {
    /// Canary item.
    const fn new() -> u8 { //~ clippy::new_ret_no_self
        1
    }
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    sink(Thing::new());
}

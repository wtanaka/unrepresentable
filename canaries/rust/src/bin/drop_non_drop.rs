//! Canary.
/// Canary item.
struct Wrapper(u8);

#[rustfmt::skip]
fn main() {
    let value = Wrapper(1);
    assert!(value.0 == 1, "one");
    drop(value); //~ clippy::drop_non_drop
}

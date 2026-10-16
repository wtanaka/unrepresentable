//! Canary.
/// Canary item.
fn sink<Ty>(_val: Ty) {}

#[rustfmt::skip]
fn main() {
    sink(|num: u8| async move { num }); //~ closure_returning_async_block
}

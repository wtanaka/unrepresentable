//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Thing;

#[rustfmt::skip]
impl ToString for Thing { //~ clippy::to_string_trait_impl
    fn to_string(&self) -> String {
        String::new()
    }
}

#[rustfmt::skip]
fn main() {
    assert!(Thing.to_string().is_empty(), "empty");
}

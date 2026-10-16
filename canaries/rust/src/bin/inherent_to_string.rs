//! Canary.
/// Canary item.
struct Thing(u8);

#[rustfmt::skip]
impl Thing {
    /// Canary item.
    fn to_string(&self) -> String { //~ clippy::inherent_to_string
        self.0.to_string()
    }
}

fn main() {
    let thing = Thing(1);
    assert!(thing.to_string().is_empty(), "empty");
}

//! Canary.
/// Canary item.
trait Speak {
    #[inline] //~ clippy::inline_fn_without_body unused_attributes
    /// Canary item.
    fn speak(&self);
}

/// Canary item.
struct Dog;

impl Speak for Dog {
    fn speak(&self) {}
}

fn main() {
    Dog.speak();
}

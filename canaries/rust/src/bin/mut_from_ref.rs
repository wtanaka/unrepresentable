//! Canary.
#[rustfmt::skip]
/// Canary item.
trait Getter { //~ dead_code
    /// Canary item.
    fn get(&self, val: &String) -> &mut String; //~ clippy::mut_from_ref clippy::ptr_arg
}
#[rustfmt::skip]
fn main() {}

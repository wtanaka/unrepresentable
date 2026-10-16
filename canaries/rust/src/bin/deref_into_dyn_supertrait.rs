//! Canary.
use core::ops::Deref;

/// Canary item.
trait Sup {}

/// Canary item.
trait Sub: Sup {}

#[rustfmt::skip]
impl Deref for dyn Sub { //~ deref_into_dyn_supertrait
    type Target = dyn Sup;
    fn deref(&self) -> &Self::Target {
        self
    }
}

fn main() {}

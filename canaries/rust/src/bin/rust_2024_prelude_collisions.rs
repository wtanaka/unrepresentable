//! Canary.
use core::future::{self, Future};

/// Canary item.
trait Ext {
    /// Canary item.
    fn into_future(self);
}

impl<Fut: Future> Ext for Fut {
    fn into_future(self) {}
}

fn main() {
    future::ready::<u8>(1).into_future(); //~ rust_2024_prelude_collisions
}

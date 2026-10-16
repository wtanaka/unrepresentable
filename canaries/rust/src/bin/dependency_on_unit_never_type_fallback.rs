//! Canary.
use std::env;

/// Canary item.
fn make<Ty>() -> Result<Ty, ()>
where
    Ty: Default,
{
    if env::args().count() > 5 {
        Err(())
    } else {
        Ok(Ty::default())
    }
}

#[rustfmt::skip]
/// Canary item.
fn run() -> Result<(), ()> { //~ dependency_on_unit_never_type_fallback
    make()?;
    Ok(())
}

fn main() {
    assert!(run().is_ok(), "ok");
}

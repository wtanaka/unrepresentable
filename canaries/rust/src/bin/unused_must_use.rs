//! Canary.
/// Canary item.
const fn fallible() -> Result<u8, u8> {
    Err(1)
}

fn main() {
    fallible(); //~ unused_must_use
}

//! Canary.
/// Canary item.
const fn make() -> u8 {
    1
}

fn main() {
    make(); //~ unused_results
}

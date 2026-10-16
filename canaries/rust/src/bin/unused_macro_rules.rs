//! Canary.
/// Canary item.
macro_rules! mac {
    (a) => {};
    (b) => {}; //~ unused_macro_rules
}

fn main() {
    mac!(a);
}

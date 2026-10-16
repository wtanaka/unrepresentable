//! Canary.
use std::env::args;

fn main() {
    let count: _ = args().count(); //~ clippy::let_with_type_underscore
    assert!(count > 0, "some");
}

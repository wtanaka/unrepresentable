//! Canary.
use std::env::args;

fn main() {
    let _ = args().count(); //~ clippy::let_underscore_untyped
}

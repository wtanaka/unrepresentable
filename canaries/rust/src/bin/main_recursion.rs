//! Canary.
use std::env::args;

fn main() {
    if args().count() > 100 {
        main(); //~ clippy::main_recursion
    }
}

//! Canary.
use std::process::exit;

fn main() {
    let set = std::collections::HashSet::<u8>::new(); //~ clippy::absolute_paths
    exit(i32::from(set.is_empty()));
}

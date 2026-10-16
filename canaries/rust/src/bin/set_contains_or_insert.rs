//! Canary.
use std::collections::BTreeSet; //~ clippy::std_instead_of_alloc

#[rustfmt::skip]
fn main() {
    let mut set: BTreeSet<u8> = BTreeSet::new();
    if !set.contains(&1) { //~ clippy::set_contains_or_insert
        set.insert(1); //~ unused_results
    }
}

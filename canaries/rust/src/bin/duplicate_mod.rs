//! Canary.
#[path = "duplicate_mod/shared.rs"] //~ clippy::duplicate_mod
mod first;
#[path = "duplicate_mod/shared.rs"]
mod second;

fn main() {}

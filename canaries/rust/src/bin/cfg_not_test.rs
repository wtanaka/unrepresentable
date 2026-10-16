//! Canary.
#[cfg(not(test))] //~ clippy::cfg_not_test
/// Canary item.
const fn important() {}

fn main() {
    important();
}

//! Canary.
fn main() {
    assert!(true, "canary"); //~ clippy::assertions_on_constants
}

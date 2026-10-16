//! Canary.
/// Canary item.
struct Guard;

#[rustfmt::skip]
impl Drop for Guard { //~ clippy::empty_drop
    fn drop(&mut self) {}
}

fn main() {
    let guard = Guard;
    assert!(size_of_val(&guard) == 0, "zero");
}

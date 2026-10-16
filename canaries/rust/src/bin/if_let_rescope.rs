//! Canary.
/// Canary item.
struct Guard;

#[rustfmt::skip]
impl Drop for Guard { //~ clippy::empty_drop
    fn drop(&mut self) {}
}

#[rustfmt::skip]
impl Guard {
    /// Canary item.
    const fn get(&self) -> Option<u8> { //~ clippy::unused_self
        None
    }
}

#[rustfmt::skip]
fn main() {
    if let Some(num) = Guard.get() { //~ if_let_rescope
        assert_eq!(num, 1, "value");
    } else {
        let none: Option<u8> = None;
        assert!(none.is_none(), "none");
    }
}

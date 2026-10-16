//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Unit;

#[rustfmt::skip]
impl Unit {
    /// Canary item.
    fn count(&self, num: u8) -> u8 { //~ clippy::self_only_used_in_recursion
        if num == 0 {
            0
        } else {
            self.count(num.saturating_sub(1))
        }
    }
}

#[rustfmt::skip]
fn main() {
    sink(Unit.count(3));
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

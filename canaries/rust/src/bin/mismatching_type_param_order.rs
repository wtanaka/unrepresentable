//! Canary.
#[derive(Debug)]
/// Canary item.
struct Pair<Left, Right>(Left, Right);

#[rustfmt::skip]
impl<Right, Left> Pair<Right, Left> { //~ clippy::mismatching_type_param_order
    /// Canary item.
    const fn first(&self) -> &Right {
        &self.0
    }
}

fn main() {
    let (one, two): (u8, u8) = (1, 2);
    let pair = Pair(one, two);
    assert_eq!(*pair.first(), 1, "first");
}

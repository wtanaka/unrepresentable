//! Canary.
/// Canary item.
enum Wrapper {
    /// Canary item.
    Only(u8),
}

#[rustfmt::skip]
fn main() {
    let wrapped = Wrapper::Only(1);
    let inner = match wrapped { //~ clippy::infallible_destructuring_match
        Wrapper::Only(inner) => inner,
    };
    assert_eq!(inner, 1, "same");
}

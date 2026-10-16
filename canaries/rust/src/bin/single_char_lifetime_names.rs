//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Holder<'a> { //~ clippy::single_char_lifetime_names
    /// Canary item.
    text: &'a str,
}

#[rustfmt::skip]
fn main() {
    let holder = Holder { text: "x" };
    assert!(holder.text.is_empty(), "empty");
}

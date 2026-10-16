//! Canary.
#[inline(always)] //~ clippy::inline_always
/// Canary item.
fn dup(val: &str) -> String {
    val.to_owned()
}

fn main() {
    assert!(dup("a").is_empty(), "empty");
}

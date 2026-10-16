//! Canary.
#[rustfmt::skip]
fn main() {
    let mut text = String::new();
    text = text + "a"; //~ clippy::string_add_assign clippy::assign_op_pattern
    assert!(!text.is_empty(), "text");
}

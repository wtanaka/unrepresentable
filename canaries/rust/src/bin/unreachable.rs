//! Canary.
/// Canary item.
fn pick(src: i32) -> i32 {
    if src.is_positive() {
        1
    } else {
        unreachable!("never") //~ clippy::unreachable
    }
}
fn main() {
    let _v = pick(1).to_string();
}

//! Canary.
fn main() {
    let src: (i32, i32, i32) = (1, 2, 3);
    let (first, .., _) = src; //~ clippy::unneeded_wildcard_pattern
    let _v = first.to_string();
}

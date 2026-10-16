//! Canary.
fn main() {
    let src: Vec<i32> = Vec::new();
    let iter = src.iter().peekable(); //~ clippy::unused_peekable
    let _v = iter.count();
}

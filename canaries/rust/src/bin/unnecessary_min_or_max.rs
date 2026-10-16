//! Canary.
fn main() {
    let src = u8::MAX;
    let _v = src.min(u8::MAX); //~ clippy::unnecessary_min_or_max
}

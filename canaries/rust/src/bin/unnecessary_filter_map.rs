//! Canary.
fn main() {
    let src: [i32; 2] = [1, 2];
    let iter = src.iter();
    let _v = iter.filter_map(|item| Some(item.is_positive())); //~ clippy::unnecessary_filter_map
}

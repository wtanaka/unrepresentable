//! Canary.
fn main() {
    let src: [i32; 2] = [1, 2];
    let _v: Option<bool> = src.iter().find_map(|item| Some(item.is_positive())); //~ clippy::unnecessary_find_map
}

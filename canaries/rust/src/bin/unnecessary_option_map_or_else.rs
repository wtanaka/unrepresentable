//! Canary.
fn main() {
    let src: Option<bool> = None;
    let _v: bool = src.map_or_else(|| false, |item| item); //~ clippy::unnecessary_option_map_or_else
}

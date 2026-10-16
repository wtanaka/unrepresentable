//! Canary.
fn main() {
    let data: Vec<String> = Vec::new();
    let copy: Vec<String> = data.iter().cloned().collect(); //~ clippy::iter_cloned_collect
    assert_eq!(copy, data, "same");
}

//! Canary.
fn main() {
    let data: Vec<String> = Vec::new();
    let num = data.iter().cloned().filter(String::is_empty).count(); //~ clippy::iter_overeager_cloned
    assert_eq!(num, 0, "empty");
}

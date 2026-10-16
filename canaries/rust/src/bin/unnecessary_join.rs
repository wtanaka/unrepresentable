//! Canary.
fn main() {
    let src: Vec<String> = Vec::new();
    let _v = src.iter().map(ToString::to_string).collect::<Vec<String>>().join(""); //~ clippy::unnecessary_join
}

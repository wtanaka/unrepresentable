//! Canary.
fn main() {
    let src: Vec<i32> = Vec::new();
    for (_, item) in src.iter().enumerate() { //~ clippy::unused_enumerate_index
        let _v = item.to_string();
    }
}

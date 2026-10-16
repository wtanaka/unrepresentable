//! Canary.
fn main() {
    let src: Vec<bool> = Vec::new();
    let mut iter = src.iter();
    while let Some(item) = iter.next() { //~ clippy::while_let_on_iterator
        let _v = item.to_string();
    }
}

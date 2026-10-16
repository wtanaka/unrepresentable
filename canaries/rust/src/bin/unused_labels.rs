//! Canary.
fn main() {
    let items: [u8; 1] = [1];
    'label: for _ in items {} //~ unused_labels
}

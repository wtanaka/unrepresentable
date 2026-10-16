//! Canary.
use core::hint::black_box;

fn main() {
    let mut text = String::new(); //~ unused_mut
    let _: usize = black_box(text.len());
}

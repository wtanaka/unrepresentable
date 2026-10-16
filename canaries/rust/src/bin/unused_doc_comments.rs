//! Canary.
#[rustfmt::skip]
use core::hint::black_box;

#[rustfmt::skip]
fn main() {
    #[doc = "x"] let _unused: u8 = black_box(1); //~ unused_doc_comments
}

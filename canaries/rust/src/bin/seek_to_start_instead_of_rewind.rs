//! Canary.
use std::io::{Cursor, Seek as _, SeekFrom};

#[rustfmt::skip]
fn main() {
    let mut cur = Cursor::new(Vec::<u8>::new());
    cur.seek(SeekFrom::Start(0)); //~ clippy::seek_to_start_instead_of_rewind unused_must_use
}

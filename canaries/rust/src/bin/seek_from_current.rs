//! Canary.
use std::io::{Cursor, Seek as _, SeekFrom};

#[rustfmt::skip]
fn main() {
    let mut cur = Cursor::new(Vec::<u8>::new());
    drop(cur.seek(SeekFrom::Current(0))); //~ clippy::seek_from_current
}

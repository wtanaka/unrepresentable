//! Canary.
use std::path::PathBuf;

#[rustfmt::skip]
fn main() {
    let mut buf = PathBuf::new(); //~ clippy::pathbuf_init_then_push
    buf.push("a");
    drop(buf);
}

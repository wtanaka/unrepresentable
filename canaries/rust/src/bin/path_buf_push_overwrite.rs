//! Canary.
use std::path::PathBuf;

#[rustfmt::skip]
fn main() {
    let mut buf = PathBuf::from("/a"); //~ clippy::pathbuf_init_then_push
    buf.push("/b"); //~ clippy::path_buf_push_overwrite
    drop(buf);
}

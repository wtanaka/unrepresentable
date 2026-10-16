//! Canary.
use std::fs::metadata;

#[rustfmt::skip]
fn main() {
    if let Ok(meta) = metadata(".") {
        assert!(!meta.file_type().is_file(), "directory"); //~ clippy::filetype_is_file
    }
}

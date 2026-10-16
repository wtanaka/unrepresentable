//! Canary.
use std::path::Path;

fn main() {
    let path = Path::new("a").join("/b"); //~ clippy::join_absolute_paths
    assert!(path.is_absolute(), "abs");
}

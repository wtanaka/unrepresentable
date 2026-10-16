//! Canary.
use std::process::exit;

fn main() {
    exit(i32::from("name".ends_with(".rs"))); //~ clippy::case_sensitive_file_extension_comparisons
}

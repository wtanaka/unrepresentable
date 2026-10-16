//! Canary.
use std::process::exit;

fn main() {
    let text = String::new();
    let sl: &[String] = &[text.clone()]; //~ clippy::cloned_ref_to_slice_refs
    exit(i32::from(sl.is_empty() && text.is_empty()));
}

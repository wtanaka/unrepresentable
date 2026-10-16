//! Canary.
use std::process::exit;

#[rustfmt::skip]
fn main() {
    let opt: Option<Option<u8>> = None;
    match opt { //~ clippy::single_match
        Some(inner) => match inner { //~ clippy::collapsible_match clippy::single_match
            Some(val) => exit(i32::from(val)),
            None => {}
        },
        None => {}
    }
}

//! Canary.
use std::process::Command;
fn main() {
    let _child = Command::new("true").spawn().expect("spawn"); //~ clippy::zombie_processes clippy::expect_used
}

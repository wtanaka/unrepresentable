//! Canary.
use std::process::Command;

#[rustfmt::skip]
fn main() {
    let mut command = Command::new("echo");
    let argued = command.arg("-n hello"); //~ clippy::suspicious_command_arg_space
    assert!(argued.get_args().count() > 0, "args");
}

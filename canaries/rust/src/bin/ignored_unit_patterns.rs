//! Canary.
use std::process::exit;

fn main() {
    let res: Result<(), i32> = Ok(());
    let code: i32 = match res {
        Ok(_) => 0, //~ clippy::ignored_unit_patterns
        Err(err) => err,
    };
    exit(code);
}

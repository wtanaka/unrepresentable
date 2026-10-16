//! `unrepresentable`: an opinionated AST linter that rejects Rust code whose types and signatures allow invalid states.

extern crate alloc;

mod against;
mod analyze;
mod attrs;
mod baseline;
mod canaries;
mod cli;
mod config;
mod failure;
mod findings;
mod git;
mod glob;
mod guards;
mod index;
mod io;
mod json;
mod knobs;
mod ledger;
mod lintcov;
mod loader;
mod process;
mod rules;
mod scan;
mod selftest;
mod toml;
mod types;
mod util;
mod workspace;

use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    match cli::run(args) {
        Ok(status) => match status {
            cli::ExitStatus::Clean => ExitCode::SUCCESS,
            cli::ExitStatus::Findings => ExitCode::from(1),
            cli::ExitStatus::Approval => ExitCode::from(3),
        },
        Err(failure) => {
            let mut text = failure.text().as_str().to_owned();
            text.push('\n');
            match io::write_stderr(&types::Message::from_string(text)) {
                Ok(()) | Err(_) => ExitCode::from(2),
            }
        }
    }
}

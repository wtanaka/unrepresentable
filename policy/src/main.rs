//! The merge-policy binary: classifies a change, evaluates approvals, posts
//! check runs and audits repository settings.

/// Makes the `alloc` crate available.
extern crate alloc;

#[macro_use]
mod json;

#[cfg(test)]
#[macro_use]
mod testkit;

#[cfg(test)]
mod workflow_checks;

mod approval;
mod artifacts;
mod audit;
mod bytes;
mod checkrun;
mod classify;
mod cli;
mod codeowners;
mod data;
mod differential;
mod exit;
mod failure;
mod fetchhead;
mod github;
mod headjobs;
mod installer;
mod installtool;
mod io;
mod post;
mod process;
mod pushskip;
mod reason;
mod releasegate;
mod render;
mod report;
mod review;
mod selflint;
mod timestamp;
mod toml;
mod types;
mod verdict;

use std::env;
use std::process::ExitCode;

use crate::exit::ExitStatus;
use crate::types::Text;

fn main() -> ExitCode {
    let args: Vec<Text> = env::args_os()
        .skip(1)
        .map(|item| Text::from_string(item.to_string_lossy().into_owned()))
        .collect();
    match cli::run(&args, &io::read_vars()) {
        Ok(ExitStatus::Clean) => ExitCode::SUCCESS,
        Ok(ExitStatus::Failed) => ExitCode::from(1),
        Err(failure) => {
            let line = Text::from_string(format!("{}\n", failure.text().as_str()));
            match io::write_stderr(&line) {
                Ok(()) | Err(_) => ExitCode::from(2),
            }
        }
    }
}

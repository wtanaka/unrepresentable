//! Command dispatch: one subcommand per job step.
//!
//! ```text
//! unrepresentable-policy classify --head-dir D --base SHA --head SHA [--zizmor-json F] --out F
//! unrepresentable-policy post decide <results-dir> <base-dir>
//! unrepresentable-policy post merge
//! unrepresentable-policy review [--base-dir D] [--head-sha SHA] [PR_NUMBER]
//! unrepresentable-policy push-skip <commit>
//! unrepresentable-policy release-gate <commit>
//! unrepresentable-policy audit <settings.json> [drift-file]
//! unrepresentable-policy differential fetch-public <corpus.lock> <dest>
//! unrepresentable-policy differential fetch-hidden <dest>
//! unrepresentable-policy differential run --base-bin B --head-bin H --prefix P
//!     --base-dir D --base SHA --head SHA --out F
//!     [--head-input name=dir ...] [--input name=dir ...] [--hidden-input name=dir]
//! unrepresentable-policy self-lint --prefix P <base-binary> <head-dir> <base-dir> <base> <head> <out>
//! unrepresentable-policy head-jobs mutants <mutants.out> <exit> <base> <head> <out> <pr.diff>
//! unrepresentable-policy head-jobs checks <status-file> <base> <head> <out>
//! unrepresentable-policy fetch-head <dest-dir> <base-dir>
//! unrepresentable-policy install-tool <tools.lock> <name> <dest-dir>
//! unrepresentable-policy lock-check <Cargo.lock>
//! unrepresentable-policy make-installer --template F --sums F --version V --out F
//! ```

pub(crate) mod args;
mod jobs;
mod policy;

#[cfg(test)]
mod tests;

use crate::exit::{ExitStatus, Report};
use crate::failure::Failure;
use crate::io::{self, Vars};
use crate::types::{Count, Message, Text};
use args::Arguments;

/// Positional argument positions.
pub(crate) const FIRST: Count = Count::new(0);

/// The second positional argument.
pub(crate) const SECOND: Count = Count::new(1);
/// The third positional argument.
pub(crate) const THIRD: Count = Count::new(2);
/// The fourth positional argument.
pub(crate) const FOURTH: Count = Count::new(3);
/// The fifth positional argument.
pub(crate) const FIFTH: Count = Count::new(4);
/// The sixth positional argument.
pub(crate) const SIXTH: Count = Count::new(5);
/// The seventh positional argument.
pub(crate) const SEVENTH: Count = Count::new(6);

/// The failure for a missing or unknown command.
fn usage_failure() -> Failure {
    Failure::usage(&Message::of(
        "usage: unrepresentable-policy <classify|post|review|push-skip|release-gate|audit|differential|self-lint|head-jobs|fetch-head|install-tool|lock-check|make-installer> ...",
    ))
}

/// Runs the command named by `args` and prints what it reports.
pub(crate) fn run(args: &[Text], vars: &Vars) -> Result<ExitStatus, Failure> {
    let Some((command, rest)) = args.split_first() else {
        return Err(usage_failure());
    };
    let report = dispatch(command, &Arguments::parse(rest)?, vars)?;
    for line in &report.lines {
        io::write_stdout(&Text::from_string(format!("{}\n", line.as_str())))?;
    }
    Ok(report.status)
}

/// Runs the command named `command`.
fn dispatch(command: &Text, args: &Arguments, vars: &Vars) -> Result<Report, Failure> {
    match command.as_str() {
        "classify" => jobs::classify(args),
        "post" => policy::post(args, vars),
        "review" => policy::review(args, vars),
        "push-skip" => policy::push_skip(args, vars),
        "release-gate" => policy::release_gate(args, vars),
        "audit" => policy::audit(args, vars),
        "differential" => jobs::differential(args, vars),
        "self-lint" => jobs::self_lint(args),
        "head-jobs" => jobs::head_jobs(args),
        "fetch-head" => jobs::fetch_head(args, vars),
        "install-tool" => jobs::install_tool(args),
        "lock-check" => jobs::lock_check(args),
        "make-installer" => jobs::make_installer(args),
        _ => Err(usage_failure()),
    }
}

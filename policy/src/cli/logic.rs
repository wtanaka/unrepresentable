use super::args::Arguments;
use crate::exit::{ExitStatus, Report};
use crate::failure::Failure;
use crate::io::{self, Vars};
use crate::types::{Count, Message, Text};

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
        "classify" => super::jobs::classify(args),
        "post" => super::policy::post(args, vars),
        "review" => super::policy::review(args, vars),
        "push-skip" => super::policy::push_skip(args, vars),
        "release-gate" => super::policy::release_gate(args, vars),
        "audit" => super::policy::audit(args, vars),
        "differential" => super::jobs::differential(args, vars),
        "self-lint" => super::jobs::self_lint(args),
        "head-jobs" => super::jobs::head_jobs(args),
        "fetch-head" => super::jobs::fetch_head(args, vars),
        "install-tool" => super::jobs::install_tool(args),
        "lock-check" => super::jobs::lock_check(args),
        "make-installer" => super::jobs::make_installer(args),
        _ => Err(usage_failure()),
    }
}

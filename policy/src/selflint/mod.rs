//! Run the BASE binary on the HEAD tree as data.
//!
//! The base binary runs `cargo clippy` on the head tree, and cargo runs what
//! the tree tells it to: a `build.rs`, a `.cargo/config.toml` with a
//! `rustc-wrapper`, a `rust-toolchain.toml` naming a toolchain by path, a
//! procedural macro. So the binary only runs behind a sandbox prefix (no
//! network, no secrets, an unprivileged user, a toolchain and crates the
//! caller staged), on a tree made read-only, with a build directory of its
//! own. Nothing of the head is executed outside that sandbox.
//!
//! Run A: head tree + BASE config and ledgers (so head cannot grant itself
//! exceptions): any finding fails the job.
//! Run B: head tree as-is with `check --against <base>`: `approval:` lines become
//! reasons (`CONFIG_LOOSENED`, `LEDGER_ROW_ADDED`, `BASELINE_GREW`, ...).

#[cfg(test)]
mod tests;

use crate::artifacts;
use crate::bytes::Bytes;
use crate::exit::Report;
use crate::failure::Failure;
use crate::io::{self, CopyPlan};
use crate::process::{self, Args, EnvPair, Invocation, SandboxUser, Status, BASE_UID};
use crate::reason::{Code, Commits, Counts, Job, Reason};
use crate::types::{Answer, Count, Message, Name, RelPath, Text};
use std::path::{Path, PathBuf};

/// The files copied from the head over the base before the run.
const OVERLAY_FILES: [&str; 3] = [
    ".unrepresentable/config",
    ".unrepresentable/ledger.md",
    ".unrepresentable/canary-exemptions.md",
];

/// The prefix of an approval line in the tool output.
const APPROVAL_PREFIX: &str = "approval: ";
/// The most characters of evidence that are kept.
const EVIDENCE_LIMIT: usize = 300;
/// The most bytes of output that are read.
const OUTPUT_LIMIT: usize = 200_000;
/// The most bytes of output that are shown.
const SHOWN_LIMIT: usize = 20_000;
/// The exit code that stands for a signal.
const SIGNAL_CODE: usize = 255;
/// The exit code that means the change needs approval.
const NEEDS_APPROVAL: usize = 3;

/// What to run and where.
pub(crate) struct SelfLint {
    /// The directory of the base checkout.
    pub base_dir: PathBuf,
    /// The binary to run.
    pub binary: PathBuf,
    /// The base and head commits.
    pub commits: Commits,
    /// The directory of the head checkout.
    pub head_dir: PathBuf,
    /// Where to write the result.
    pub out: PathBuf,
    /// The sandbox words to put in front of each run of the binary.
    pub prefix: Vec<Text>,
}

/// A run's exit code and its output.
struct Ran {
    /// The exit code.
    code: Count,
    /// What the run printed.
    output: Text,
}

/// The reason an `approval:` line stands for.
pub(crate) fn map_approval(text: &Text) -> Reason {
    let low = text.as_str().to_lowercase();
    let has = |word: &str| low.contains(word);
    let code = if has("baseline") {
        Code::BaselineGrew
    } else if has("ledger") || has("exemption") || has("expect") {
        Code::LedgerRowAdded
    } else if has("config") || has("loosen") || has("lint") {
        Code::ConfigLoosened
    } else {
        Code::ToolApproval
    };
    let evidence: String = text.as_str().chars().take(EVIDENCE_LIMIT).collect();
    Reason::general(code, &Text::from_string(evidence))
}

/// The reasons for every `approval:` line in the tool's output.
pub(crate) fn parse_approvals(output: &Text) -> Vec<Reason> {
    output
        .as_str()
        .lines()
        .filter_map(|line| line.strip_prefix(APPROVAL_PREFIX))
        .map(|rest| map_approval(&Text::of(rest.trim())))
        .collect()
}

/// Runs the binary behind the sandbox prefix and collects its code and output.
/// `tree` is the directory it is run on: the sandboxed user may not own it, so
/// git is told to trust it. The binary builds in a scratch directory of its own
/// that the sandbox gives to the user it runs as, and that is removed
/// afterwards when it can be.
fn run_binary(request: &SelfLint, args: &Args, tree: &Path) -> Result<Ran, Failure> {
    let build = process::make_scratch_dir()?;
    let pair = |name: &str, value: &str| EnvPair {
        name: Name::of(name),
        value: Text::of(value),
    };
    let env = [
        pair("CARGO_TARGET_DIR", &build.to_string_lossy()),
        pair("GIT_CONFIG_COUNT", "1"),
        pair("GIT_CONFIG_KEY_0", "safe.directory"),
        pair("GIT_CONFIG_VALUE_0", &tree.to_string_lossy()),
    ];
    let invocation = Invocation::new(&Name::of(&request.binary.to_string_lossy()), args)
        .in_dir(tree)
        .behind(&request.prefix, &env)
        .run_as(&SandboxUser {
            scratch: build.clone(),
            uid: BASE_UID,
        });
    let captured = process::run(&invocation);
    // Files the sandbox user made in the directory are not the caller's to
    // remove, and the runner is discarded after the job: a failure to clean up
    // is not a failure of the run.
    if let Err(cleanup) = io::remove_tree(&build) {
        io::write_stderr(&Text::from_string(format!(
            "note: the scratch directory was not removed: {}\n",
            cleanup.text().as_str()
        )))?;
    }
    let done = captured?;
    let code = match done.status {
        Status::Code(found) => found,
        Status::Signal => Count::new(SIGNAL_CODE),
    };
    Ok(Ran {
        code,
        output: Text::from_string(
            done.combined()
                .as_str()
                .chars()
                .take(OUTPUT_LIMIT)
                .collect(),
        ),
    })
}

/// Output made safe for a log: printable ASCII and newlines only.
fn printable(text: &Text) -> Text {
    Text::from_string(
        text.as_str()
            .chars()
            .map(|found| {
                if found == '\n' || (' '..='~').contains(&found) {
                    found
                } else {
                    '?'
                }
            })
            .take(SHOWN_LIMIT)
            .collect(),
    )
}

/// Run A: the head tree extracted next to the base config and ledgers, as a
/// repository of one commit (the tool needs a change range, and `HEAD` selects
/// no change, so only the scan is judged).
fn run_a(request: &SelfLint) -> Result<Ran, Failure> {
    let work = process::make_temp_dir()?;
    let result = overlay_and_run(request, &work);
    process::make_writable(&work)?;
    io::remove_tree(&work)?;
    result
}

/// Refuses when `name` under `work`, or any directory on the way to it, is a
/// link. The head tree was extracted from the head's archive, so it can hold a
/// link where the base's tool file goes (`.unrepresentable` or
/// `.unrepresentable/config` leading to a file of the runner), and copying or
/// removing through it would change that file as the runner user.
fn refuse_links(work: &Path, name: &RelPath) -> Result<(), Failure> {
    let mut here = work.to_path_buf();
    for part in name.as_str().split('/') {
        here.push(part);
        if io::is_link(&here) == Answer::Yes {
            return Err(Failure::refused(&Message::from_string(format!(
                "the head has a link on the way to {}; the tool files of the base cannot be placed over it",
                name.as_str()
            ))));
        }
    }
    Ok(())
}

/// Copies the overlay files in and runs the binary in `work`.
fn overlay_and_run(request: &SelfLint, work: &Path) -> Result<Ran, Failure> {
    let archive = process::git(
        &request.head_dir,
        &Args::of(&["archive", request.commits.head.as_str()]),
    )?;
    let tar = Name::of("tar");
    process::run(
        &Invocation::new(&tar, &Args::of(&["-x", "-C", &work.to_string_lossy()])).feeding(&archive),
    )?
    .stdout_or_fail(&tar)
    .map(|_output: Bytes| ())?;
    for name in OVERLAY_FILES {
        let source = request.base_dir.join(name);
        let target = work.join(name);
        refuse_links(work, &RelPath::of(name))?;
        match (io::is_file(&source), io::exists_no_follow(&target)) {
            (Answer::Yes, Answer::Yes | Answer::No) => io::copy_file(&CopyPlan {
                from: source,
                to: target,
            })?,
            (Answer::No, Answer::Yes) => io::remove_file(&target)?,
            (Answer::No, Answer::No) => {}
        }
    }
    process::init_repository(work)?;
    process::make_read_only(work)?;
    run_binary(
        request,
        &Args::of(&["check", "--range", "HEAD", &work.to_string_lossy()]),
        work,
    )
}

/// Runs both checks and writes the result artifact.
pub(crate) fn run(request: &SelfLint) -> Result<Report, Failure> {
    let mut lines = Vec::new();
    let mut failed = false;
    let mut reasons = Vec::new();
    let first = run_a(request)?;
    if first.code.get() != 0 {
        failed = true;
        lines.push(Text::from_string(format!(
            "run A (base rules and ledgers over head tree) exit {}",
            first.code.get()
        )));
        lines.push(printable(&first.output));
    }
    process::make_read_only(&request.head_dir)?;
    let second = run_binary(
        request,
        &Args::of(&["check", "--against", request.commits.base.as_str()])
            .with(&request.head_dir.to_string_lossy()),
        &request.head_dir,
    )?;
    match second.code.get() {
        NEEDS_APPROVAL => reasons.extend(parse_approvals(&second.output)),
        0 => {}
        other => {
            failed = true;
            lines.push(Text::from_string(format!(
                "run B (check --against) exit {other}"
            )));
            lines.push(printable(&second.output));
        }
    }
    let counts = Counts::empty()
        .with(&Name::of("run_a_exit"), first.code)
        .with(&Name::of("run_b_exit"), second.code);
    let text = artifacts::emit(Job::SelfLint, &request.commits, &reasons, &counts);
    io::write_text(&request.out, &text)?;
    Ok(if failed {
        Report::failed(lines)
    } else {
        Report {
            lines,
            ..Report::clean()
        }
    })
}

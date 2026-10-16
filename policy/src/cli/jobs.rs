//! The subcommands that prepare inputs and results for the policy decision.

use super::args::Arguments;
use super::{FIFTH, FIRST, FOURTH, SECOND, SIXTH, THIRD};
use crate::artifacts;
use crate::classify::{self, manifest};
use crate::differential::{self, corpus, workspace, Input, Origin, TargetDirs};
use crate::exit::Report;
use crate::failure::Failure;
use crate::fetchhead::{self, FetchHead};
use crate::headjobs::{self, ChecksRun, MutantsRun};
use crate::installer::{self, Installer};
use crate::installtool::{self, Install};
use crate::io::{self, Vars};
use crate::json;
use crate::process;
use crate::reason::{Commits, Counts, Job, Reason};
use crate::selflint::{self, SelfLint};
use crate::types::{Answer, Count, Holds as _, Message, Name, Text};
use std::path::{Path, PathBuf};

/// The most bytes read from a lock file.
const LOCK_LIMIT: usize = 5_000_000;

/// The job, reasons and counts of a differential artifact.
struct Artifact {
    /// The counts.
    counts: Counts,
    /// The job that emits the artifact.
    job: Job,
    /// The reasons.
    reasons: Vec<Reason>,
}

/// The prepared inputs, or the report that ends the run.
type Workspaces = Result<Vec<Input>, Report>;

/// The `name=dir` entries of one kind of input, with the origin they have.
struct Specs {
    /// The `name=dir` entries.
    entries: Vec<Text>,
    /// Where inputs of this kind come from.
    origin: Origin,
}

/// `classify --head-dir D --base SHA --head SHA [--zizmor-json F] --out F`.
pub(crate) fn classify(args: &Arguments) -> Result<Report, Failure> {
    let commits = Commits {
        base: args.commit(&Name::of("base"))?,
        head: args.commit(&Name::of("head"))?,
    };
    let zizmor = args
        .optional(&Name::of("zizmor-json"))
        .map(|found| PathBuf::from(found.as_str()));
    let classified = classify::run(&classify::Request {
        commits: commits.clone(),
        dir: args.path(&Name::of("head-dir"))?,
        zizmor,
    })?;
    let text = artifacts::emit(
        Job::Classify,
        &commits,
        &classified.reasons,
        &classified.counts,
    );
    io::write_text(&args.path(&Name::of("out"))?, &text)?;
    Ok(Report::clean())
}

/// `self-lint --prefix <sandbox words> <base-binary> <head-dir> <base-dir> <base> <head> <out>`.
pub(crate) fn self_lint(args: &Arguments) -> Result<Report, Failure> {
    let prefix = sandbox_prefix(args)?;
    selflint::run(&SelfLint {
        base_dir: args.word_path(THIRD)?,
        binary: args.word_path(FIRST)?,
        commits: Commits {
            base: args.word_commit(FOURTH)?,
            head: args.word_commit(FIFTH)?,
        },
        head_dir: args.word_path(SECOND)?,
        out: args.word_path(SIXTH)?,
        prefix,
    })
}

/// `head-jobs mutants|checks ...`.
pub(crate) fn head_jobs(args: &Arguments) -> Result<Report, Failure> {
    let usage = || Failure::usage(&Message::of("head-jobs needs mutants or checks"));
    let which = args.word(FIRST)?;
    match which.as_str() {
        "mutants" => {
            let exit = Count::parse(&args.word(THIRD)?).ok_or_else(usage)?;
            headjobs::mutants(&MutantsRun {
                commits: Commits {
                    base: args.word_commit(FOURTH)?,
                    head: args.word_commit(FIFTH)?,
                },
                exit,
                out: args.word_path(SIXTH)?,
                out_dir: args.word_path(SECOND)?,
            })
        }
        "checks" => headjobs::checks(&ChecksRun {
            commits: Commits {
                base: args.word_commit(THIRD)?,
                head: args.word_commit(FOURTH)?,
            },
            out: args.word_path(FIFTH)?,
            status_file: args.word_path(SECOND)?,
        }),
        _ => Err(usage()),
    }
}

/// `fetch-head <dest-dir> <base-dir>`.
pub(crate) fn fetch_head(args: &Arguments, vars: &Vars) -> Result<Report, Failure> {
    fetchhead::run(
        &FetchHead {
            base_dir: args.word_path(SECOND)?,
            dest: args.word_path(FIRST)?,
        },
        vars,
    )?;
    Ok(Report::clean())
}

/// `install-tool <tools.lock> <name> <dest-dir>`.
pub(crate) fn install_tool(args: &Arguments) -> Result<Report, Failure> {
    installtool::install(&Install {
        dest: args.word_path(THIRD)?,
        lock: args.word_path(FIRST)?,
        tool: Name::of(args.word(SECOND)?.as_str()),
    })
}

/// `lock-check <Cargo.lock>`: clean only when every package is from crates.io
/// with a checksum.
pub(crate) fn lock_check(args: &Arguments) -> Result<Report, Failure> {
    let text =
        io::read_bytes_limited(&args.word_path(FIRST)?, Count::new(LOCK_LIMIT))?.lossy_text();
    Ok(match manifest::lock_is_registry_only(&text)? {
        Answer::Yes => Report::clean(),
        Answer::No => Report::clean().into_failed(),
    })
}

/// `make-installer --template F --sums F --version V --out F`.
pub(crate) fn make_installer(args: &Arguments) -> Result<Report, Failure> {
    let text = installer::generate(&Installer {
        sums: io::read_text(&args.path(&Name::of("sums"))?)?,
        template: io::read_text(&args.path(&Name::of("template"))?)?,
        version: args.required(&Name::of("version"))?,
    })?;
    io::write_text(&args.path(&Name::of("out"))?, &text)?;
    Ok(Report::clean())
}

/// `differential fetch-public|fetch-hidden|run ...`.
pub(crate) fn differential(args: &Arguments, vars: &Vars) -> Result<Report, Failure> {
    let usage = || {
        Failure::usage(&Message::of(
            "differential needs fetch-public, fetch-hidden or run",
        ))
    };
    match args.word(FIRST)?.as_str() {
        "fetch-public" => {
            corpus::fetch_public(&corpus::PublicCorpus {
                dest: args.word_path(THIRD)?,
                lock: args.word_path(SECOND)?,
            })?;
            Ok(Report::clean())
        }
        "fetch-hidden" => {
            corpus::fetch_hidden(vars, &args.word_path(SECOND)?)?;
            Ok(Report::clean())
        }
        "run" => run_differential(args),
        _ => Err(usage()),
    }
}

/// The inputs of a differential run, each made a read-only workspace with two
/// writable scratch build directories; a bad spec, or a hidden input next to
/// any other input, ends the run with exit 2. The hidden corpus is compared in
/// a run of its own, so that nothing from the pull request is ever handed to
/// the job that holds its secret.
fn workspaces(kinds: &[Specs]) -> Result<Workspaces, Failure> {
    let hidden = kinds
        .iter()
        .any(|kind| kind.origin == Origin::Hidden && !kind.entries.is_empty());
    let others = kinds
        .iter()
        .any(|kind| kind.origin != Origin::Hidden && !kind.entries.is_empty());
    if hidden && others {
        return Ok(Err(Report::saying(&Text::of(
            "a --hidden-input cannot be combined with any other input",
        ))
        .into_failed()));
    }
    let mut inputs = Vec::new();
    for kind in kinds {
        for spec in &kind.entries {
            let (name, path) = match spec.as_str().split_once('=') {
                Some((before, after)) => (before, after),
                None => (spec.as_str(), ""),
            };
            let name_ok = !name.is_empty()
                && name.len() <= 24
                && name.chars().all(|found| {
                    found.is_ascii_lowercase() || found.is_ascii_digit() || found == '-'
                });
            let dir = PathBuf::from(path);
            if !name_ok || io::is_dir(&dir) == Answer::No {
                return Ok(Err(Report::saying(&Text::of("bad input")).into_failed()));
            }
            let root = workspace::make_workspace(&dir, kind.origin)?;
            process::make_read_only(&root)?;
            inputs.push(Input {
                name: Name::of(name),
                origin: kind.origin,
                root,
                targets: TargetDirs {
                    base: process::make_scratch_dir()?,
                    head: process::make_scratch_dir()?,
                },
            });
        }
    }
    Ok(Ok(inputs))
}

/// The sandbox words every differential run needs: both binaries run inputs
/// that may hold code.
fn sandbox_prefix(args: &Arguments) -> Result<Vec<Text>, Failure> {
    let prefix: Vec<Text> = args
        .required(&Name::of("prefix"))?
        .as_str()
        .split_whitespace()
        .map(Text::of)
        .collect();
    if prefix.is_empty() {
        return Err(Failure::usage(&Message::of(
            "--prefix must name the sandbox command",
        )));
    }
    Ok(prefix)
}

/// The three kinds of input a differential run takes.
fn input_kinds(args: &Arguments) -> [Specs; 3] {
    [
        Specs {
            entries: args.all(&Name::of("head-input")),
            origin: Origin::Head,
        },
        Specs {
            entries: args.all(&Name::of("hidden-input")),
            origin: Origin::Hidden,
        },
        Specs {
            entries: args.all(&Name::of("input")),
            origin: Origin::Public,
        },
    ]
}

/// The artifact a comparison gives: the hidden corpus is a job of its own and
/// is reported as a number only.
fn differential_result(
    comparison: &differential::Comparison,
    hidden_run: Answer,
    base_dir: &Path,
) -> Result<Artifact, Failure> {
    Ok(match hidden_run {
        Answer::Yes => Artifact {
            counts: comparison
                .counts
                .clone()
                .with(&Name::of("lost_hidden"), comparison.hidden_lost),
            job: Job::DifferentialHidden,
            reasons: differential::hidden_reasons(comparison.hidden_lost),
        },
        Answer::No => Artifact {
            counts: comparison
                .counts
                .clone()
                .with(&Name::of("lost"), Count::new(comparison.lost.len())),
            job: Job::Differential,
            reasons: differential::to_reasons(&comparison.lost, base_dir)?,
        },
    })
}

/// Runs the differential command. The sandbox prefix is required.
fn run_differential(args: &Arguments) -> Result<Report, Failure> {
    let commits = Commits {
        base: args.commit(&Name::of("base"))?,
        head: args.commit(&Name::of("head"))?,
    };
    let prefix = sandbox_prefix(args)?;
    let inputs = match workspaces(&input_kinds(args))? {
        Ok(found) => found,
        Err(report) => return Ok(report),
    };
    if inputs.is_empty() {
        return Ok(Report::saying(&Text::of("no input")).into_failed());
    }
    let tools = differential::Tools {
        base: args.path(&Name::of("base-bin"))?,
        head: args.path(&Name::of("head-bin"))?,
        prefix,
    };
    let comparison = differential::compare(&tools, &inputs)?;
    let hidden_run = inputs
        .iter()
        .any(|input| input.origin == Origin::Hidden)
        .answer();
    let artifact =
        differential_result(&comparison, hidden_run, &args.path(&Name::of("base-dir"))?)?;
    let text = artifacts::emit(artifact.job, &commits, &artifact.reasons, &artifact.counts);
    io::write_text(&args.path(&Name::of("out"))?, &text)?;
    Ok(if comparison.errors.is_empty() {
        Report::clean()
    } else {
        Report::failed(comparison.errors)
    })
}

/// A JSON document read from a file.
pub(crate) fn read_json(path: &Path) -> Result<json::Json, Failure> {
    json::parse(&io::read_text(path)?).map_err(|message| Failure::parse(&message))
}

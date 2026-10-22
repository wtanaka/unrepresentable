//! Shared helpers for the integration tests: scratch workspaces, a real git
//! repository, and the `unrepresentable` binary under test.
//!
//! Fixture sources live under `tests/fixtures/` with an `.in` suffix so that
//! neither cargo nor the tool itself mistakes them for code or config of this
//! repository. They are copied into a scratch workspace with the suffix
//! removed. Expected findings live under `tests/expected/`.

use core::fmt;

use std::ffi::OsString;

use std::fs;

use std::io;

#[cfg(unix)]
use std::os::unix::fs::symlink;

use std::path::{Path, PathBuf};

use std::process::{Command, ExitStatus, Output};

/// Fails the enclosing `Result`-returning check unless `condition` holds.
macro_rules! ensure {
    ($condition:expr_2021, $($note:tt)+) => {
        if !$condition {
            return Err($crate::support::Fail::new($crate::support::Message::from_string(
                format!($($note)+),
            )));
        }
    };
}

/// Fails the enclosing `Result`-returning check unless both sides are equal.
macro_rules! ensure_eq {
    ($left:expr_2021, $right:expr_2021, $($note:tt)+) => {{
        let left = $left;
        let right = $right;
        if left != right {
            return Err($crate::support::Fail::new($crate::support::Message::from_string(
                format!("{}\n  left: {:?}\n right: {:?}", format!($($note)+), left, right),
            )));
        }
    }};
}

/// Files copied from this repository into every scratch workspace.
const LINT_FILES: [&str; 4] = [
    "clippy.toml",
    "rust-toolchain.toml",
    ".unrepresentable/ledger.md",
    ".unrepresentable/canary-exemptions.md",
];

/// Rules that are notes, not findings: they never change the exit code.
const NOTE_RULES: &[&str] = &["preference-omission"];

/// A free-form piece of text: file contents, a git command line, an output line.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Message(String);

impl Message {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) const fn from_string(text: String) -> Self {
        Self(text)
    }

    pub(crate) fn of(text: &str) -> Self {
        Self(text.to_owned())
    }
}

/// A short identifier: a test case, a package, a branch.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Name(String);

impl Name {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) const fn from_string(text: String) -> Self {
        Self(text)
    }

    pub(crate) fn of(text: &str) -> Self {
        Self(text.to_owned())
    }
}

/// A path relative to a scratch workspace, `/`-separated.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RelPath(String);

impl RelPath {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) const fn from_string(text: String) -> Self {
        Self(text)
    }

    pub(crate) fn of(text: &str) -> Self {
        Self(text.to_owned())
    }
}

/// Why a test could not even run.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Fail(Message);

impl Fail {
    pub(crate) const fn new(message: Message) -> Self {
        Self(message)
    }
}

impl fmt::Display for Fail {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(self.0.as_str())
    }
}

impl From<io::Error> for Fail {
    fn from(error: io::Error) -> Self {
        Self(Message::from_string(format!("io error: {error}")))
    }
}

/// The answer to a yes/no question about a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    No,
    Yes,
}

/// Turns a condition into an [`Answer`].
pub(crate) trait Holds {
    fn answer(self) -> Answer;
}

impl Holds for bool {
    fn answer(self) -> Answer {
        if self {
            Answer::Yes
        } else {
            Answer::No
        }
    }
}

/// How the binary exited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Exit {
    Approval,
    Clean,
    Error,
    Findings,
    Other,
}

impl Exit {
    fn of_status(status: ExitStatus) -> Self {
        match status.code() {
            Some(0) => Self::Clean,
            Some(1) => Self::Findings,
            Some(2) => Self::Error,
            Some(3) => Self::Approval,
            Some(_) | None => Self::Other,
        }
    }
}

/// One command-line option of the binary. `Canaries`, `Init` and `SelfTest`
/// select the subcommand; with none of them the subcommand is `check`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Flag {
    Against(Name),
    Canaries,
    Init,
    Range(Message),
    Ratchet,
    /// Any other argument, passed through untouched.
    Raw(Message),
    SelfTest,
}

/// Everything one run of the binary produced.
#[derive(Clone, Debug)]
pub(crate) struct Outcome {
    pub exit: Exit,
    pub stderr: Message,
    pub stdout: Message,
}

/// A textual replacement in one file of a scratch workspace.
#[derive(Clone, Debug)]
pub(crate) struct Edit {
    pub file: RelPath,
    pub from: Message,
    pub to: Message,
}

impl Outcome {
    /// The `approval:` reasons, in output order.
    pub(crate) fn approvals(&self) -> Vec<Message> {
        self.stdout
            .as_str()
            .lines()
            .filter_map(|line| line.strip_prefix("approval: "))
            .map(Message::of)
            .collect()
    }

    /// `path:line:col: rule` for every finding, in output order.
    pub(crate) fn findings(&self) -> Vec<Message> {
        let mut found = Vec::new();
        for line in self.stdout.as_str().lines() {
            if line.starts_with("approval: ") || line.starts_with("range: ") {
                continue;
            }
            let mut parts = line.splitn(3, ": ");
            if let (Some(place), Some(rule), Some(_)) = (parts.next(), parts.next(), parts.next()) {
                if !NOTE_RULES.contains(&rule) {
                    found.push(Message::from_string(format!("{place}: {rule}")));
                }
            }
        }
        found
    }

    /// Does any approval reason start with `prefix`?
    pub(crate) fn has_approval(&self, prefix: &Message) -> Answer {
        self.approvals()
            .iter()
            .any(|reason| reason.as_str().starts_with(prefix.as_str()))
            .answer()
    }

    /// Is there a finding of `rule` in `path` (any line and column)?
    pub(crate) fn has_rule_in(&self, path: &RelPath, rule: &Name) -> Answer {
        let start = format!("{}:", path.as_str());
        let end = format!(": {}", rule.as_str());
        self.findings()
            .iter()
            .any(|line| line.as_str().starts_with(&start) && line.as_str().ends_with(&end))
            .answer()
    }

    pub(crate) const fn stderr(&self) -> &Message {
        &self.stderr
    }

    pub(crate) const fn stdout(&self) -> &Message {
        &self.stdout
    }
}

/// A symbolic link to create: where it is, and what it points to (relative to
/// the link's own directory).
#[cfg(unix)]
#[derive(Clone, Debug)]
pub(crate) struct Link {
    pub path: RelPath,
    pub to: RelPath,
}

/// Stand-ins for the `$CARGO` and `$RUSTC` executables of one run; `None` keeps
/// the default.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Programs<'path> {
    /// The executable to use as `$CARGO`.
    pub cargo: Option<&'path Path>,
    /// Whether the run has `CARGO_BUILD_JOBS=1` or inherits the environment.
    pub jobs: BuildJobs,
    /// The executable to use as `$RUSTC`.
    pub rustc: Option<&'path Path>,
}

impl Programs<'_> {
    /// Keeps both `$CARGO` and `$RUSTC` at their defaults, with one compile job.
    pub(crate) const fn none() -> Self {
        Self {
            cargo: None,
            jobs: BuildJobs::One,
            rustc: None,
        }
    }
}

/// A scratch directory with a workspace (and optionally a git repository) in it.
#[derive(Clone, Debug)]
pub(crate) struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    /// Appends `extra` to the end of `path`.
    pub(crate) fn append(&self, path: &RelPath, extra: &Message) -> Result<(), Fail> {
        let text = self.read(path)?;
        self.write(
            path,
            &Message::from_string(format!("{}{}", text.as_str(), extra.as_str())),
        )
    }

    /// Stages everything and commits with `subject` (one word).
    pub(crate) fn commit_all(&self, subject: &Name) -> Result<(), Fail> {
        self.git(&Message::of("add -A"))?;
        self.git(&Message::from_string(format!(
            "commit -q --allow-empty -m {}",
            subject.as_str()
        )))
    }

    /// Copies every file directly inside this repository's `dir` directory
    /// into the same directory of the workspace.
    pub(crate) fn copy_repo_files(&self, dir: &RelPath) -> Result<(), Fail> {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir.as_str());
        for entry in fs::read_dir(source)? {
            let item = entry?;
            let name = item.file_name().to_string_lossy().into_owned();
            let text = fs::read_to_string(item.path())?;
            self.write(
                &RelPath::from_string(format!("{}/{name}", dir.as_str())),
                &Message::from_string(text),
            )?;
        }
        Ok(())
    }

    /// A fresh, empty scratch directory unique to `name`.
    pub(crate) fn create(name: &Name) -> Result<Self, Fail> {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("unrepresentable-tests")
            .join(name.as_str());
        if root.exists() {
            fs::remove_dir_all(&root)?;
        }
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// The default fixture configuration: a pure package, no baseline.
    pub(crate) fn default_config(package: &Name) -> Message {
        Message::from_string(format!(
            "baseline = \"forbidden\"\ndefault-range = \"HEAD\"\ncode = [\"src/**\"]\n\n[member.{}]\nrole = \"pure\"\n",
            package.as_str()
        ))
    }

    /// Replaces the first occurrence of `from` in `path`; fails when absent.
    pub(crate) fn edit(&self, edit: &Edit) -> Result<(), Fail> {
        let text = self.read(&edit.file)?;
        if !text.as_str().contains(edit.from.as_str()) {
            return Err(Fail::new(Message::from_string(format!(
                "{} does not contain {:?}",
                edit.file.as_str(),
                edit.from.as_str()
            ))));
        }
        let changed = text
            .as_str()
            .replacen(edit.from.as_str(), edit.to.as_str(), 1);
        self.write(&edit.file, &Message::from_string(changed))
    }

    /// Runs `git <line>` (split on whitespace) in the workspace; fails on a non-zero exit.
    pub(crate) fn git(&self, line: &Message) -> Result<(), Fail> {
        git_in(line, &self.root)
    }

    /// Is there a file or directory at `path`?
    pub(crate) fn has_path(&self, path: &RelPath) -> Answer {
        self.root.join(path.as_str()).exists().answer()
    }

    /// `git init`, then commit every file on branch `main`.
    pub(crate) fn init_repo(&self) -> Result<(), Fail> {
        self.git(&Message::of("init -q -b main"))?;
        self.commit_all(&Name::of("base"))
    }

    /// Writes an executable `cargo` stand-in that appends its arguments to
    /// `cargo-calls.log` and then runs the real cargo; returns its path.
    pub(crate) fn install_cargo_logger(&self) -> Result<RelPath, Fail> {
        let path = RelPath::of("cargo-logger.sh");
        self.write(
            &path,
            &Message::of(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$(dirname \"$0\")/cargo-calls.log\"\nexec cargo \"$@\"\n",
            ),
        )?;
        let output = Command::new("chmod")
            .arg("+x")
            .arg(self.root.join(path.as_str()))
            .output()?;
        ensure!(output.status.success(), "chmod +x failed");
        Ok(path)
    }

    /// Writes an executable `rustc` stand-in whose `--print sysroot` answers
    /// `sysroot`, a directory inside the workspace that the caller creates;
    /// returns the stand-in's path.
    #[cfg(unix)]
    pub(crate) fn install_fake_rustc(&self, sysroot: &RelPath) -> Result<RelPath, Fail> {
        let path = RelPath::of("fake-rustc.sh");
        self.write(
            &path,
            &Message::from_string(format!(
                "#!/bin/sh\nprintf '%s\\n' \"$(pwd)/{}\"\n",
                sysroot.as_str()
            )),
        )?;
        let output = Command::new("chmod")
            .arg("+x")
            .arg(self.root.join(path.as_str()))
            .output()?;
        ensure!(output.status.success(), "chmod +x failed");
        Ok(path)
    }

    /// Creates the symbolic link.
    #[cfg(unix)]
    pub(crate) fn link(&self, link: &Link) -> Result<(), Fail> {
        let target = self.root.join(link.path.as_str());
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        symlink(link.to.as_str(), target)?;
        Ok(())
    }

    /// Copies `tests/fixtures/<fixture>/` over the workspace, dropping the
    /// `.in` suffix; `#LINTS#` in a `Cargo.toml.in` becomes the lint tables.
    pub(crate) fn overlay(&self, fixture: &RelPath) -> Result<(), Fail> {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(fixture.as_str());
        let lints = lint_tables()?;
        copy_tree(&source, &lints, &self.root)
    }

    pub(crate) fn read(&self, path: &RelPath) -> Result<Message, Fail> {
        Ok(Message::from_string(fs::read_to_string(
            self.root.join(path.as_str()),
        )?))
    }

    pub(crate) fn remove(&self, path: &RelPath) -> Result<(), Fail> {
        fs::remove_file(self.root.join(path.as_str()))?;
        Ok(())
    }

    /// Runs the binary on the workspace root.
    pub(crate) fn run(&self, flags: &[Flag]) -> Result<Outcome, Fail> {
        self.run_args(flags, Argument::Root, Programs::none())
    }

    fn run_args(
        &self,
        flags: &[Flag],
        mode: Argument,
        programs: Programs<'_>,
    ) -> Result<Outcome, Fail> {
        let root = self.root.as_os_str().to_owned();
        let mut args: Vec<OsString> = vec![match mode {
            Argument::SelfTest => OsString::from("self-test"),
            Argument::Root | Argument::RootFirst => subcommand_of(flags),
        }];
        if mode == Argument::RootFirst {
            args.push(root.clone());
        }
        args.extend(flag_args(flags));
        match mode {
            Argument::Root => args.push(root),
            Argument::RootFirst | Argument::SelfTest => {}
        }
        let output = spawn(Program::Tool, &args, &self.root, programs)?;
        Ok(Outcome {
            exit: Exit::of_status(output.status),
            stderr: Message::from_string(String::from_utf8_lossy(&output.stderr).into_owned()),
            stdout: Message::from_string(String::from_utf8_lossy(&output.stdout).into_owned()),
        })
    }

    /// Like [`Sandbox::run`] but with `$CARGO` set to the executable `cargo`,
    /// a path inside the workspace.
    pub(crate) fn run_with_cargo(&self, flags: &[Flag], cargo: &RelPath) -> Result<Outcome, Fail> {
        let program = self.root.join(cargo.as_str());
        let programs = Programs {
            cargo: Some(&program),
            jobs: BuildJobs::One,
            rustc: None,
        };
        self.run_args(flags, Argument::Root, programs)
    }

    /// Like [`Sandbox::run`] but with `CARGO_BUILD_JOBS` left as the environment
    /// has it, so cargo may compile in parallel.
    pub(crate) fn run_with_default_jobs(&self, flags: &[Flag]) -> Result<Outcome, Fail> {
        let programs = Programs {
            cargo: None,
            jobs: BuildJobs::Inherited,
            rustc: None,
        };
        self.run_args(flags, Argument::Root, programs)
    }

    /// Like [`Sandbox::run`] but with the root before the flags, so that a
    /// trailing flag has no value.
    pub(crate) fn run_with_root_first(&self, flags: &[Flag]) -> Result<Outcome, Fail> {
        self.run_args(flags, Argument::RootFirst, Programs::none())
    }

    /// Like [`Sandbox::run`] but with `$RUSTC` set to the executable `rustc`,
    /// a path inside the workspace.
    pub(crate) fn run_with_rustc(&self, flags: &[Flag], rustc: &RelPath) -> Result<Outcome, Fail> {
        let program = self.root.join(rustc.as_str());
        let programs = Programs {
            cargo: None,
            jobs: BuildJobs::One,
            rustc: Some(&program),
        };
        self.run_args(flags, Argument::Root, programs)
    }

    /// Writes a minimal single-package workspace that satisfies every guard
    /// except the rules a fixture means to exercise: this repository's lint
    /// tables, clippy.toml, toolchain pin and ledgers, plus `config`.
    pub(crate) fn seed(&self, package: &Name, config: &Message) -> Result<(), Fail> {
        let manifest = format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lints]\nworkspace = true\n\n[workspace]\nmembers = [\".\"]\n\n{}",
            package.as_str(),
            lint_tables()?.as_str()
        );
        self.write(&RelPath::of("Cargo.toml"), &Message::from_string(manifest))?;
        let lock = format!(
            "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"{}\"\nversion = \"0.1.0\"\n",
            package.as_str()
        );
        self.write(&RelPath::of("Cargo.lock"), &Message::from_string(lock))?;
        for raw in LINT_FILES {
            let file = RelPath::of(raw);
            self.write(&file, &repo_file(&file)?)?;
        }
        self.write(&RelPath::of(".unrepresentable/config"), config)?;
        self.write(&RelPath::of(".gitignore"), &Message::of("target/\n"))?;
        Ok(())
    }

    /// Like [`Sandbox::seed`] for a virtual workspace of `crates/<name>` members,
    /// each a bin crate with an empty `main`.
    pub(crate) fn seed_members(&self, packages: &[Name], config: &Message) -> Result<(), Fail> {
        let manifest = format!(
            "[workspace]\nmembers = [\"crates/*\"]\n\n{}",
            lint_tables()?.as_str()
        );
        self.write(&RelPath::of("Cargo.toml"), &Message::from_string(manifest))?;
        let mut lock = String::from(
            "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n",
        );
        let mut sorted: Vec<&Name> = packages.iter().collect();
        sorted.sort();
        for package in sorted {
            lock = format!(
                "{lock}\n[[package]]\nname = \"{}\"\nversion = \"0.1.0\"\n",
                package.as_str()
            );
            self.write(
                &RelPath::from_string(format!("crates/{}/Cargo.toml", package.as_str())),
                &Message::from_string(format!(
                    "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lints]\nworkspace = true\n",
                    package.as_str()
                )),
            )?;
            self.write(
                &RelPath::from_string(format!("crates/{}/src/main.rs", package.as_str())),
                &Message::of("fn main() {}\n"),
            )?;
        }
        self.write(&RelPath::of("Cargo.lock"), &Message::from_string(lock))?;
        for raw in LINT_FILES {
            let file = RelPath::of(raw);
            self.write(&file, &repo_file(&file)?)?;
        }
        self.write(&RelPath::of(".unrepresentable/config"), config)?;
        self.write(&RelPath::of(".gitignore"), &Message::of("target/\n"))?;
        Ok(())
    }

    /// Runs the binary with only `self-test`.
    pub(crate) fn self_test(&self) -> Result<Outcome, Fail> {
        self.run_args(&[], Argument::SelfTest, Programs::none())
    }

    /// Clones this repository shallowly (depth 1) into a sibling scratch directory.
    pub(crate) fn shallow_clone(&self, name: &Name) -> Result<Self, Fail> {
        let target = Self::create(name)?;
        fs::remove_dir_all(&target.root)?;
        let url = format!("file://{}", self.root.display());
        git_in(
            &Message::from_string(format!(
                "clone -q --depth 1 {url} {}",
                target.root.display()
            )),
            &self.root,
        )?;
        Ok(target)
    }

    pub(crate) fn write(&self, path: &RelPath, contents: &Message) -> Result<(), Fail> {
        let target = self.root.join(path.as_str());
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, contents.as_str())?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Argument {
    Root,
    RootFirst,
    SelfTest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Program {
    Git,
    Tool,
}

/// The compile jobs a spawned run may use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuildJobs {
    /// The environment's `CARGO_BUILD_JOBS`, or cargo's default when unset.
    Inherited,
    /// `CARGO_BUILD_JOBS=1`.
    One,
}

fn repo_file(name: &RelPath) -> Result<Message, Fail> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name.as_str());
    Ok(Message::from_string(fs::read_to_string(path)?))
}

/// The `[workspace.lints.*]` tables of this repository's root manifest.
fn lint_tables() -> Result<Message, Fail> {
    let manifest = repo_file(&RelPath::of("Cargo.toml"))?;
    let mut out = String::new();
    let mut inside = false;
    for line in manifest.as_str().lines() {
        if line.starts_with('[') {
            inside = line.starts_with("[workspace.lints");
        }
        if inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    Ok(Message::from_string(out))
}

/// The subcommand the flags select: `check` unless one of them names another.
fn subcommand_of(flags: &[Flag]) -> OsString {
    for flag in flags {
        match flag {
            Flag::Canaries => return OsString::from("canaries"),
            Flag::Init => return OsString::from("init"),
            Flag::SelfTest => return OsString::from("self-test"),
            Flag::Against(_) | Flag::Range(_) | Flag::Ratchet | Flag::Raw(_) => {}
        }
    }
    OsString::from("check")
}

fn flag_args(flags: &[Flag]) -> Vec<OsString> {
    let mut out = Vec::new();
    for flag in flags {
        match flag {
            Flag::Against(reference) => {
                out.push(OsString::from("--against"));
                out.push(OsString::from(reference.as_str()));
            }
            Flag::Range(range) => {
                out.push(OsString::from("--range"));
                out.push(OsString::from(range.as_str()));
            }
            Flag::Ratchet => out.push(OsString::from("--ratchet")),
            Flag::Canaries | Flag::Init | Flag::SelfTest => {}
            Flag::Raw(text) => out.push(OsString::from(text.as_str())),
        }
    }
    out
}

fn git_in(line: &Message, dir: &Path) -> Result<(), Fail> {
    let args: Vec<OsString> = line
        .as_str()
        .split_whitespace()
        .map(OsString::from)
        .collect();
    let output = spawn(Program::Git, &args, dir, Programs::none())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Fail::new(Message::from_string(format!(
            "git {} failed: {}",
            line.as_str(),
            String::from_utf8_lossy(&output.stderr)
        ))))
    }
}

/// Runs `program` in `dir`, keeping it away from the user's git configuration
/// and giving a nested `cargo clippy` its own target directory.
fn spawn(
    program: Program,
    args: &[OsString],
    dir: &Path,
    programs: Programs<'_>,
) -> Result<Output, Fail> {
    let toolchain = repo_file(&RelPath::of("rust-toolchain.toml"))?;
    let channel = toolchain
        .as_str()
        .lines()
        .find_map(|line| line.strip_prefix("channel = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .map(OsString::from);
    let mut variables: Vec<(&str, OsString)> = vec![
        ("GIT_CONFIG_GLOBAL", OsString::from("/dev/null")),
        ("GIT_CONFIG_SYSTEM", OsString::from("/dev/null")),
        ("GIT_AUTHOR_NAME", OsString::from("tester")),
        ("GIT_AUTHOR_EMAIL", OsString::from("tester@example.invalid")),
        ("GIT_COMMITTER_NAME", OsString::from("tester")),
        (
            "GIT_COMMITTER_EMAIL",
            OsString::from("tester@example.invalid"),
        ),
        ("CARGO_TARGET_DIR", dir.join("target").into_os_string()),
    ];
    if programs.jobs == BuildJobs::One {
        // One compile job at a time for the nested `cargo clippy` canary runs,
        // whose JSON diagnostics are read line by line. The lint tables do not
        // come from cargo, so they do not depend on this.
        variables.push(("CARGO_BUILD_JOBS", OsString::from("1")));
    }
    if let Some(pinned) = channel {
        variables.push(("RUSTUP_TOOLCHAIN", pinned));
    }
    if let Some(program_path) = programs.cargo {
        variables.push(("CARGO", program_path.as_os_str().to_owned()));
    }
    if let Some(program_path) = programs.rustc {
        variables.push(("RUSTC", program_path.as_os_str().to_owned()));
    }
    let executable = match program {
        Program::Git => OsString::from("git"),
        Program::Tool => OsString::from(env!("CARGO_BIN_EXE_unrepresentable")),
    };
    Ok(Command::new(executable)
        .args(args)
        .current_dir(dir)
        .envs(variables)
        .output()?)
}

fn copy_tree(from: &Path, lints: &Message, to: &Path) -> Result<(), Fail> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let item = entry?;
        let name = item.file_name().to_string_lossy().into_owned();
        let source = item.path();
        if item.file_type()?.is_dir() {
            copy_tree(&source, lints, &to.join(&name))?;
            continue;
        }
        let target_name = match name.strip_suffix(".in") {
            Some(stripped) => stripped.to_owned(),
            None => name.clone(),
        };
        let mut text = fs::read_to_string(&source)?;
        if target_name == "Cargo.toml" {
            text = text.replace("#LINTS#", lints.as_str());
        }
        fs::write(to.join(target_name), text)?;
    }
    Ok(())
}

/// Builds the workspace for `tests/fixtures/<case>/`, commits it, runs the
/// binary against `HEAD`, and compares the findings with
/// `tests/expected/<case>.txt` (one `path:line:col: rule` per line).
pub(crate) fn check_case(case: &Name) -> Result<(), Fail> {
    let package = Name::of("fx");
    let flat = case.as_str().replace('/', "-");
    let sandbox = Sandbox::create(&Name::from_string(format!("case-{flat}")))?;
    sandbox.seed(&package, &Sandbox::default_config(&package))?;
    sandbox.overlay(&RelPath::of(case.as_str()))?;
    sandbox.init_repo()?;
    let outcome = sandbox.run(&[Flag::Range(Message::of("HEAD"))])?;
    let expected_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("expected")
        .join(format!("{}.txt", case.as_str()));
    let expected: Vec<Message> = fs::read_to_string(expected_path)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(Message::of)
        .collect();
    let want = if expected.is_empty() {
        Exit::Clean
    } else {
        Exit::Findings
    };
    ensure!(
        outcome.exit != Exit::Error,
        "{} did not run: {}",
        case.as_str(),
        outcome.stderr().as_str()
    );
    ensure_eq!(
        outcome.findings(),
        expected,
        "findings of {}",
        case.as_str()
    );
    ensure_eq!(outcome.exit, want, "exit status of {}", case.as_str());
    Ok(())
}

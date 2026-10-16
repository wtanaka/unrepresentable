//! The only module that spawns processes: `git`, `$CARGO` and `$RUSTC`. Every
//! `cargo` child gets `--release --locked`, so the tool never creates a `debug`
//! profile directory and never rewrites `Cargo.lock`.

use std::env;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::failure::Failure;
use crate::types::{Message, Source, Text};

/// The flags every `cargo` child gets right after its subcommand.
const CARGO_FLAGS: [&str; 2] = ["--release", "--locked"];

/// Command line arguments for a spawned process.
#[derive(Clone, Debug)]
pub(crate) struct Args(Vec<String>);

impl Args {
    /// Builds an argument list from string slices.
    pub(crate) fn of(items: &[&str]) -> Self {
        Self(items.iter().map(|item| (*item).to_owned()).collect())
    }

    /// Returns the list with one more argument appended.
    pub(crate) fn with(mut self, item: &str) -> Self {
        self.0.push(item.to_owned());
        self
    }
}

/// Raw bytes captured from a process.
#[derive(Clone, Debug)]
pub(crate) struct Bytes(Vec<u8>);

impl Bytes {
    /// The bytes as UTF-8, or `None` when they are not valid UTF-8.
    #[must_use]
    pub(crate) fn strict_text(&self) -> Option<Text> {
        match String::from_utf8(self.0.clone()) {
            Ok(text) => Some(Text::from_string(text)),
            Err(_) => None,
        }
    }

    /// The bytes as text, replacing invalid UTF-8 with the replacement character.
    pub(crate) fn to_text(&self) -> Text {
        Text::from_string(String::from_utf8_lossy(&self.0).into_owned())
    }
}

/// Whether a process exited successfully.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Success {
    /// The process exited with a failure status.
    No,
    /// The process exited with a success status.
    Yes,
}

/// Captured result of a finished process.
#[derive(Clone, Debug)]
pub(crate) struct Captured {
    /// Everything the process wrote to standard error, as lossy text.
    pub stderr: Source,
    /// Everything the process wrote to standard output, as raw bytes.
    pub stdout: Bytes,
    /// Whether the process exited successfully.
    pub success: Success,
}

/// The executable to spawn, as an OS string.
struct Program(OsString);

/// Runs `git` with `args` in `dir` and captures its output.
pub(crate) fn git(dir: &Path, args: &Args) -> Result<Captured, Failure> {
    run(&Program(OsString::from("git")), dir, args)
}

/// Runs `$RUSTC` (or `rustc`) on empty input, from `dir` so that its toolchain pin applies.
pub(crate) fn rustc(dir: &Path, args: &Args) -> Result<Captured, Failure> {
    let program = match env::var_os("RUSTC") {
        Some(value) => value,
        None => OsString::from("rustc"),
    };
    run(&Program(program), dir, args)
}

/// `args` with `--release` and `--locked` inserted right after the subcommand,
/// which is the first argument. Callers never pass either flag themselves.
fn cargo_args(args: &Args) -> Args {
    let mut items = args.0.iter();
    let mut out: Vec<String> = items.next().cloned().into_iter().collect();
    out.extend(CARGO_FLAGS.iter().map(|flag| (*flag).to_owned()));
    out.extend(items.cloned());
    Args(out)
}

/// Runs `$CARGO` (or `cargo`) with `args` in `dir` and captures its output.
/// The subcommand is the first of `args`; `--release --locked` follows it.
pub(crate) fn cargo(dir: &Path, args: &Args) -> Result<Captured, Failure> {
    let program = match env::var_os("CARGO") {
        Some(value) => value,
        None => OsString::from("cargo"),
    };
    run(&Program(program), dir, &cargo_args(args))
}

/// Spawns `program` with `args` in `dir`, with no stdin, and captures its output.
fn run(program: &Program, dir: &Path, args: &Args) -> Result<Captured, Failure> {
    let name: &OsStr = &program.0;
    let output = Command::new(name)
        .args(&args.0)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            Failure::git(&Message::from_string(format!(
                "cannot run {}: {error}",
                name.to_string_lossy()
            )))
        })?;
    Ok(Captured {
        stderr: Source::from_string(String::from_utf8_lossy(&output.stderr).into_owned()),
        stdout: Bytes(output.stdout),
        success: if output.status.success() {
            Success::Yes
        } else {
            Success::No
        },
    })
}

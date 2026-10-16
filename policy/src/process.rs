//! The only module that spawns processes: `git`, `gh` and the few system
//! tools (`curl`, `tar`, `chmod`, `mktemp`, `sha256sum` or `shasum`) the jobs
//! need.
//!
//! The GitHub client is `gh api`; the token goes through `GH_TOKEN` in the
//! child's environment, never argv. `curl` takes its URL and bearer token on
//! standard input as a curl config. Children have no timeout; the job
//! `timeout-minutes` cover that. There are no regular expressions and no zip,
//! tar or sha libraries: base64 comes from the `base64` crate behind
//! `bytes.rs`, archives are listed and checked with `tar -t` and `tar -tv`
//! before `tar -x` (plain files and directories only, no `..`, no absolute
//! names, a 512 MiB cap), and digests come from `sha256sum`.

use std::ffi::OsString;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::bytes::Bytes;
use crate::failure::Failure;
use crate::io::create_dirs;
use crate::types::{Answer, Count, Holds as _, Message, Name, Text};

/// The user id the base binary, and every program that is not head code run
/// for the differential check, runs as in the sandbox (`nobody`).
pub(crate) const BASE_UID: Count = Count::new(0xFFFE);
/// The user id the head binary runs as in the sandbox (65533). It is not the base
/// binary's, so neither can read, execute, signal or write what the other owns.
pub(crate) const HEAD_UID: Count = Count::new(0xFFFD);
/// The scratch subdirectories every scratch directory starts with: a home, a
/// temporary directory and a cargo home, so that a program that is given the
/// directory has nothing to create outside it.
const SCRATCH_PARTS: [&str; 3] = ["cargo-home", "home", "tmp"];

/// Command line arguments for a spawned process.
#[derive(Clone, Debug)]
pub(crate) struct Args(Vec<String>);

impl Args {
    /// No arguments.
    pub(crate) const fn empty() -> Self {
        Self(Vec::new())
    }

    /// Arguments made of `items`.
    pub(crate) fn of(items: &[&str]) -> Self {
        Self(items.iter().map(|item| (*item).to_owned()).collect())
    }

    /// These arguments followed by `more`.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn then(mut self, more: &Self) -> Self {
        self.0.extend(more.0.iter().cloned());
        self
    }

    /// The arguments with `item` added.
    pub(crate) fn with(mut self, item: &str) -> Self {
        self.0.push(item.to_owned());
        self
    }
}

/// One environment variable to set for a child.
#[derive(Clone, Debug)]
pub(crate) struct EnvPair {
    /// The name of the variable.
    pub name: Name,
    /// The value of the variable.
    pub value: Text,
}

/// The user a sandboxed program runs as and the one directory it owns. The
/// sandbox command (`policy/sandbox.sh`) reads both from its own environment
/// (`SANDBOX_UID`, `SANDBOX_SCRATCH`), gives the directory to that user
/// (mode 0700, nobody else may enter it) and runs the program as that user.
#[derive(Clone, Debug)]
pub(crate) struct SandboxUser {
    /// The directory the user owns; it must be empty and made by
    /// [`make_scratch_dir`].
    pub scratch: PathBuf,
    /// The numeric user id.
    pub uid: Count,
}

/// A process to run: program, arguments, working directory, standard input
/// and environment changes.
#[derive(Clone, Debug)]
pub(crate) struct Invocation {
    /// The arguments.
    args: Args,
    /// The directory to run in, when it is set.
    dir: Option<PathBuf>,
    /// The program to run.
    program: Name,
    /// The variables to set.
    set_env: Vec<EnvPair>,
    /// The bytes to feed to standard input, when there are any.
    stdin: Option<Bytes>,
    /// The variables to remove.
    unset_env: Vec<Name>,
}

impl Invocation {
    /// The invocation run behind the sandbox words `prefix`, with `env` set
    /// for the program only. With no prefix the variables go into the child's
    /// own environment. With a prefix they go in front of the program as
    /// `env NAME=VALUE`, because a sandbox usually clears the environment; the
    /// parent's environment is never changed either way.
    #[must_use]
    pub(crate) fn behind(self, prefix: &[Text], env: &[EnvPair]) -> Self {
        let Some((first, rest)) = prefix.split_first() else {
            let mut plain = self;
            plain.set_env.extend(env.iter().cloned());
            return plain;
        };
        let mut args = Args::empty();
        for word in rest {
            args = args.with(word.as_str());
        }
        args = args.with("env");
        for pair in env {
            args = args.with(&format!("{}={}", pair.name.as_str(), pair.value.as_str()));
        }
        args = args.with(self.program.as_str());
        for word in &self.args.0 {
            args = args.with(word);
        }
        let mut wrapped = Self::new(&Name::of(first.as_str()), &args);
        wrapped.dir = self.dir;
        wrapped.stdin = self.stdin;
        wrapped.unset_env = self.unset_env;
        wrapped
    }

    /// The invocation with `input` on standard input.
    #[must_use]
    pub(crate) fn feeding(mut self, input: &Bytes) -> Self {
        self.stdin = Some(input.clone());
        self
    }

    /// The invocation run in `dir`.
    #[must_use]
    pub(crate) fn in_dir(mut self, dir: &Path) -> Self {
        self.dir = Some(dir.to_path_buf());
        self
    }

    /// An invocation of `program` with `args`.
    pub(crate) fn new(program: &Name, args: &Args) -> Self {
        Self {
            args: args.clone(),
            dir: None,
            program: program.clone(),
            set_env: Vec::new(),
            stdin: None,
            unset_env: Vec::new(),
        }
    }

    /// The invocation run as `user`: the user id and the directory it owns are
    /// set in the environment of the process that is spawned, which is the
    /// sandbox command when there is a prefix. A sandbox clears the
    /// environment of the program, so neither reaches it.
    #[must_use]
    pub(crate) fn run_as(self, user: &SandboxUser) -> Self {
        self.with_env(&EnvPair {
            name: Name::of("SANDBOX_UID"),
            value: user.uid.text(),
        })
        .with_env(&EnvPair {
            name: Name::of("SANDBOX_SCRATCH"),
            value: Text::from_string(user.scratch.to_string_lossy().into_owned()),
        })
    }

    /// The invocation with `pair` set in the environment.
    #[must_use]
    pub(crate) fn with_env(mut self, pair: &EnvPair) -> Self {
        self.set_env.push(pair.clone());
        self
    }

    /// The invocation with `name` removed from the environment.
    #[must_use]
    pub(crate) fn without_env(mut self, name: &Name) -> Self {
        self.unset_env.push(name.clone());
        self
    }
}

/// How a child process ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    /// It exited with this code.
    Code(Count),
    /// A signal ended it.
    Signal,
}

/// Captured result of a finished process.
#[derive(Clone, Debug)]
pub(crate) struct Captured {
    /// How the process ended.
    pub status: Status,
    /// What the process wrote to standard error.
    pub stderr: Bytes,
    /// What the process wrote to standard output.
    pub stdout: Bytes,
}

impl Captured {
    /// Standard output followed by standard error, as lossy text.
    pub(crate) fn combined(&self) -> Text {
        Text::from_string(format!(
            "{}{}",
            self.stdout.lossy_text().as_str(),
            self.stderr.lossy_text().as_str()
        ))
    }

    /// The captured standard output, or a failure naming the program.
    pub(crate) fn stdout_or_fail(&self, what: &Name) -> Result<Bytes, Failure> {
        if self.succeeded() == Answer::Yes {
            Ok(self.stdout.clone())
        } else {
            Err(Failure::process(&Message::from_string(format!(
                "{} failed: {}",
                what.as_str(),
                self.stderr.lossy_text().as_str().trim()
            ))))
        }
    }

    /// Whether the process exited with code zero.
    pub(crate) fn succeeded(&self) -> Answer {
        (self.status == Status::Code(Count::new(0))).answer()
    }
}

/// Accepts the builder reference a `Command` method returns, so that no
/// result is left unused.
const fn settled(_command: &mut Command) {}

/// Runs the process to completion.
pub(crate) fn run(invocation: &Invocation) -> Result<Captured, Failure> {
    let program = OsString::from(invocation.program.as_str());
    let mut command = Command::new(&program);
    settled(command.args(&invocation.args.0));
    if let Some(dir) = &invocation.dir {
        settled(command.current_dir(dir));
    }
    for name in &invocation.unset_env {
        settled(command.env_remove(name.as_str()));
    }
    for pair in &invocation.set_env {
        settled(command.env(pair.name.as_str(), pair.value.as_str()));
    }
    settled(
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(match invocation.stdin {
                Some(_) => Stdio::piped(),
                None => Stdio::null(),
            }),
    );
    let cannot = |error: &io::Error| {
        Failure::process(&Message::from_string(format!(
            "cannot run {}: {error}",
            invocation.program.as_str()
        )))
    };
    let mut child = command.spawn().map_err(|error| cannot(&error))?;
    if let Some(input) = &invocation.stdin {
        if let Some(mut pipe) = child.stdin.take() {
            pipe.write_all(input.as_slice())
                .map_err(|error| cannot(&error))?;
        }
    }
    let output = child.wait_with_output().map_err(|error| cannot(&error))?;
    let status = match output.status.code() {
        Some(code) => match usize::try_from(code) {
            Ok(value) => Status::Code(Count::new(value)),
            Err(_) => Status::Signal,
        },
        None => Status::Signal,
    };
    Ok(Captured {
        status,
        stderr: Bytes::from_vec(output.stderr),
        stdout: Bytes::from_vec(output.stdout),
    })
}

/// `git` in `dir`, which must succeed; returns its standard output.
pub(crate) fn git(dir: &Path, args: &Args) -> Result<Bytes, Failure> {
    let program = Name::of("git");
    run(&Invocation::new(&program, args).in_dir(dir))?.stdout_or_fail(&program)
}

/// Gives `dir` a repository with one commit holding every file under it, so a
/// range such as `HEAD` resolves. The repository ignores the user's and the
/// system's git configuration, and runs no hooks.
pub(crate) fn init_repository(dir: &Path) -> Result<(), Failure> {
    let program = Name::of("git");
    let isolated = [
        EnvPair {
            name: Name::of("GIT_CONFIG_GLOBAL"),
            value: Text::of("/dev/null"),
        },
        EnvPair {
            name: Name::of("GIT_CONFIG_NOSYSTEM"),
            value: Text::of("1"),
        },
    ];
    let steps = [
        Args::of(&["init", "-q"]),
        Args::of(&["add", "--all", "--force"]),
        Args::of(&[
            "-c",
            "user.name=corpus",
            "-c",
            "user.email=corpus@invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "--no-verify",
            "--allow-empty",
            "-m",
            "corpus",
        ]),
    ];
    for args in &steps {
        let invocation = Invocation::new(&program, args)
            .in_dir(dir)
            .behind(&[], &isolated);
        run(&invocation)?
            .stdout_or_fail(&program)
            .map(|_output| ())?;
    }
    Ok(())
}

/// Makes `path` executable.
pub(crate) fn make_executable(path: &Path) -> Result<(), Failure> {
    let program = Name::of("chmod");
    let target = path.to_string_lossy().into_owned();
    run(&Invocation::new(&program, &Args::of(&["755", &target])))?
        .stdout_or_fail(&program)
        .map(|_output| ())
}

/// Makes everything under `path` readable by everyone and writable by nobody.
pub(crate) fn make_read_only(path: &Path) -> Result<(), Failure> {
    let program = Name::of("chmod");
    let target = path.to_string_lossy().into_owned();
    run(&Invocation::new(
        &program,
        &Args::of(&["-R", "a+rX,a-w", &target]),
    ))?
    .stdout_or_fail(&program)
    .map(|_output| ())
}

/// Gives the owner write permission under `path` again, so that a tree made
/// read-only for a sandboxed program can be removed.
pub(crate) fn make_writable(path: &Path) -> Result<(), Failure> {
    let program = Name::of("chmod");
    let target = path.to_string_lossy().into_owned();
    run(&Invocation::new(
        &program,
        &Args::of(&["-R", "u+rwX", &target]),
    ))?
    .stdout_or_fail(&program)
    .map(|_output| ())
}

/// A fresh directory under the system temporary directory for a sandboxed
/// program's build output: mode 0700 and owned by the caller until the sandbox
/// gives it to the user the program runs as (see [`SandboxUser`]), so no other
/// user can read or write it. It holds empty `cargo-home`, `home` and `tmp`
/// directories.
pub(crate) fn make_scratch_dir() -> Result<PathBuf, Failure> {
    let dir = make_temp_dir()?;
    let program = Name::of("chmod");
    let target = dir.to_string_lossy().into_owned();
    run(&Invocation::new(&program, &Args::of(&["700", &target])))?
        .stdout_or_fail(&program)
        .map(|_output| ())?;
    for part in SCRATCH_PARTS {
        create_dirs(&dir.join(part))?;
    }
    Ok(dir)
}

/// A fresh empty directory under the system temporary directory.
pub(crate) fn make_temp_dir() -> Result<PathBuf, Failure> {
    let program = Name::of("mktemp");
    let captured = run(&Invocation::new(&program, &Args::of(&["-d"])))?;
    let text = captured.stdout_or_fail(&program)?.lossy_text();
    Ok(PathBuf::from(text.as_str().trim()))
}

/// The sha256 of a file as 64 lowercase hex digits.
pub(crate) fn sha256_of(path: &Path) -> Result<Text, Failure> {
    let target = path.to_string_lossy().into_owned();
    let primary = Name::of("sha256sum");
    let captured = if let Ok(found) = run(&Invocation::new(&primary, &Args::of(&[&target]))) {
        found
    } else {
        let fallback = Name::of("shasum");
        run(&Invocation::new(
            &fallback,
            &Args::of(&["-a", "256", &target]),
        ))?
    };
    let text = captured
        .stdout_or_fail(&Name::of("sha256sum"))?
        .lossy_text();
    match text.as_str().split_whitespace().next() {
        Some(digest) => Ok(Text::of(digest)),
        None => Err(Failure::process(&Message::of("sha256sum printed nothing"))),
    }
}

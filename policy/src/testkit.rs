//! Helpers shared by the unit tests: a test failure type and the `ensure!`
//! macros, so that a check can return `Result` instead of asserting midway.

use crate::failure::Failure;
use crate::github::{Github, Method};
use crate::io;
use crate::json::{self, Json};
use crate::process::{self, Args, Invocation};
use crate::types::{Message, Name, RelPath, Sha, Text};
use core::cell::RefCell;
use core::cmp::Reverse;
use core::fmt;
use std::path::{Path, PathBuf};

/// A commit id made of one repeated digit: `sha!('a')`.
macro_rules! sha {
    ($digit:literal) => {
        $crate::types::Sha::filled($crate::types::Glyph::new($digit))
    };
}

/// Fails the enclosing `Result`-returning check unless `condition` holds.
macro_rules! ensure {
    ($condition:expr_2021, $($note:tt)+) => {
        if !$condition {
            return Err($crate::testkit::Fail::new(&$crate::types::Message::from_string(
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
            return Err($crate::testkit::Fail::new(&$crate::types::Message::from_string(
                format!("{}\n  left: {:?}\n right: {:?}", format!($($note)+), left, right),
            )));
        }
    }};
}

/// Command-line settings that make commits work anywhere.
const GIT_IDENTITY: [&str; 6] = [
    "-c",
    "user.name=t",
    "-c",
    "user.email=t@t",
    "-c",
    "commit.gpgsign=false",
];

/// Why a check could not even run, or what it found wrong.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Fail(Message);

impl Fail {
    pub(crate) fn new(message: &Message) -> Self {
        Self(message.clone())
    }
}

impl fmt::Display for Fail {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(self.0.as_str())
    }
}

impl From<Failure> for Fail {
    fn from(failure: Failure) -> Self {
        Self(failure.text())
    }
}

/// A scratch directory holding a git repository, removed when dropped.
pub(crate) struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    /// Stages everything and commits it with `message`.
    pub(crate) fn commit_all(&self, message: &Message) -> Result<(), Fail> {
        self.git(&Args::of(&["add", "-A"]))?;
        self.git(&Args::of(&["commit", "-q", "-m", message.as_str()]))?;
        Ok(())
    }

    /// Runs `git` here and discards its output.
    pub(crate) fn git(&self, args: &Args) -> Result<(), Fail> {
        self.git_text(args).map(|_output| ())
    }

    /// Runs `git` here with a fixed test identity; returns its output.
    pub(crate) fn git_text(&self, args: &Args) -> Result<Text, Fail> {
        let full = Args::of(&GIT_IDENTITY).then(args);
        Ok(process::git(&self.dir, &full)?.lossy_text())
    }

    /// The commit `HEAD` names.
    pub(crate) fn head(&self) -> Result<Sha, Fail> {
        let text = self.git_text(&Args::of(&["rev-parse", "HEAD"]))?;
        match Sha::parse(&Text::of(text.as_str().trim())) {
            Some(found) => Ok(found),
            None => Err(Fail::new(&Message::of("rev-parse printed no commit id"))),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.dir
    }

    pub(crate) fn remove(&self, relative: &RelPath) -> Result<(), Fail> {
        io::remove_file(&self.dir.join(relative.as_str()))?;
        Ok(())
    }

    /// An empty directory with `git init` run in it, on branch `main`.
    pub(crate) fn repository() -> Result<Self, Fail> {
        let scratch = Self {
            dir: process::make_temp_dir()?,
        };
        scratch.git(&Args::of(&["init", "-q", "-b", "main"]))?;
        Ok(scratch)
    }

    /// Makes `link` a symbolic link to `target`.
    pub(crate) fn symlink(&self, target: &Text, link: &RelPath) -> Result<(), Fail> {
        let program = Name::of("ln");
        let args = Args::of(&["-s", target.as_str(), link.as_str()]);
        process::run(&Invocation::new(&program, &args).in_dir(&self.dir))?
            .stdout_or_fail(&program)
            .map(|_output| ())
            .map_err(Fail::from)
    }

    pub(crate) fn write(&self, relative: &RelPath, contents: &Text) -> Result<(), Fail> {
        io::write_text(&self.dir.join(relative.as_str()), contents)?;
        Ok(())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(failure) = io::remove_tree(&self.dir) {
            note_leftover(&self.dir, &failure);
        }
    }
}

/// One write the code under test sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Sent {
    pub body: Json,
    pub method: Method,
    pub path: Text,
}

/// Whether the fake accepts writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Writes {
    Accepted,
    Refused,
}

/// An in-memory GitHub: canned replies by path fragment (the longest matching
/// fragment is used), and a record of every write.
pub(crate) struct FakeGithub {
    raw: Text,
    routes: Vec<(Text, Json)>,
    sent: RefCell<Vec<Sent>>,
    writes: Writes,
}

impl FakeGithub {
    pub(crate) fn new() -> Self {
        Self {
            raw: Text::of(""),
            routes: Vec::new(),
            sent: RefCell::new(Vec::new()),
            writes: Writes::Accepted,
        }
    }

    #[must_use]
    pub(crate) fn refusing_writes(self) -> Self {
        Self {
            writes: Writes::Refused,
            ..self
        }
    }

    #[must_use]
    pub(crate) fn route(mut self, fragment: &Text, reply: &Json) -> Self {
        self.routes.push((fragment.clone(), reply.clone()));
        self
    }

    pub(crate) fn sent(&self) -> Vec<Sent> {
        self.sent.borrow().clone()
    }

    #[must_use]
    pub(crate) fn with_text_reply(self, raw: &Text) -> Self {
        Self {
            raw: raw.clone(),
            ..self
        }
    }
}

impl Github for FakeGithub {
    fn get(&self, path: &Text) -> Result<Json, Failure> {
        let mut matching: Vec<&(Text, Json)> = self
            .routes
            .iter()
            .filter(|(fragment, _)| path.as_str().contains(fragment.as_str()))
            .collect();
        matching.sort_by_key(|(fragment, _)| Reverse(fragment.as_str().len()));
        match matching.first() {
            Some((_, reply)) => Ok(reply.clone()),
            None => Err(Failure::api(&Message::from_string(format!(
                "unexpected GET {}",
                path.as_str()
            )))),
        }
    }

    fn get_text(&self, _path: &Text) -> Result<Text, Failure> {
        Ok(self.raw.clone())
    }

    fn graphql(&self, query: &Text, variables: &Json) -> Result<Json, Failure> {
        self.send(
            Method::Post,
            &Text::of("/graphql"),
            &Json::object_of(vec![
                (Name::of("query"), Json::text_of(query)),
                (Name::of("variables"), variables.clone()),
            ]),
        )
    }

    fn repo(&self) -> Text {
        Text::of("o/r")
    }

    fn send(&self, method: Method, path: &Text, body: &Json) -> Result<Json, Failure> {
        self.sent.borrow_mut().push(Sent {
            body: body.clone(),
            method,
            path: path.clone(),
        });
        match self.writes {
            Writes::Accepted => Ok(Json::Null),
            Writes::Refused => Err(Failure::api(&Message::of("HTTP 405"))),
        }
    }
}

/// Parses JSON written in a test.
pub(crate) fn json_of(text: &Text) -> Result<Json, Fail> {
    json::parse(text).map_err(|message| Fail::new(&message))
}

/// Says on standard error that `dir` could not be removed. A drop has no way to
/// return an error, so when standard error cannot be written either, the write's
/// failure is dropped by name.
fn note_leftover(dir: &Path, failure: &Failure) {
    let note = Text::from_string(format!(
        "note: scratch directory {} was not removed: {}\n",
        dir.display(),
        failure.text().as_str()
    ));
    match io::write_stderr(&note) {
        Ok(()) => {}
        Err(unwritable) => drop(unwritable),
    }
}

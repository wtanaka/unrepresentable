//! Shared helpers for the integration tests: scratch workspaces, a real git
//! repository, and the `unrepresentable` binary under test.
//!
//! Fixture sources live under `tests/fixtures/` with an `.in` suffix so that
//! neither cargo nor the tool itself mistakes them for code or config of this
//! repository. They are copied into a scratch workspace with the suffix
//! removed. Expected findings live under `tests/expected/`.

/// The definitions of this module.
#[macro_use]
mod logic;

pub(crate) use logic::{
    check_case, pinned_channel, Answer, Edit, Exit, Fail, Flag, Holds, Link, Message, Name,
    Outcome, RelPath, Sandbox,
};

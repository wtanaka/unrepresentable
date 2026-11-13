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

/// The definitions of this module.
mod logic;

pub(crate) use logic::{run, SelfLint};

#[cfg(test)]
pub(crate) use logic::{map_approval, parse_approvals};

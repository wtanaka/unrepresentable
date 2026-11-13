//! Differential check: every finding the BASE binary reports on an input must
//! also be reported by the HEAD binary (gaining findings is fine).
//!
//! Orchestration is base code. Both binaries only ever run through a sandbox
//! prefix (no network, no tokens, empty environment, unprivileged user) on
//! read-only inputs, because an input can come from a pull request and a
//! build script or cargo configuration in it would otherwise run as the
//! runner. An input from the pull request also never keeps its own
//! configuration or manifests (see [`workspace`]).
//!
//! The two binaries run as different users ([`BASE_UID`] and [`HEAD_UID`]),
//! each with a scratch directory of its own that only that user can enter
//! and that is made fresh for every input. The head binary can therefore
//! neither run the base binary (the workflow keeps it where the head user
//! cannot read it) nor leave a file for the base binary's next run. This
//! keeps a head binary that notices the comparison from handing the work to
//! the base binary. It does not stop a head binary that carries the base
//! rules itself and behaves differently when it recognises the comparison:
//! no black-box comparison can. That is why a change under `src/` needs a
//! human (`RULES_CODE_CHANGED`) and the comparison is a second check for a
//! change that loses findings by mistake.
//!
//! The hidden corpus is secret. It is compared in a run of its own, and a
//! finding lost on it is reported only as a number, so a pull request cannot
//! read the corpus out of the findings it makes the head lose.

pub(crate) mod corpus;
pub(crate) mod workspace;

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{
    compare, hidden_reasons, to_reasons, Comparison, Input, Origin, TargetDirs, Tools,
};

#[cfg(test)]
pub(crate) use logic::{lost, parse_output, rule_source, FindingKey, Findings, LostFinding};

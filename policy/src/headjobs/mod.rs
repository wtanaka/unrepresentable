//! Turn the (untrusted) output of head-code jobs into strict result
//! artifacts. Head code may write false results; these results are necessary, not sufficient.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{checks, mutants, ChecksRun, MutantsRun};

#[cfg(test)]
pub(crate) use logic::{touched_of, Touched, EXPECTED_CHECKS};

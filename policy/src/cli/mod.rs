//! Command dispatch: one subcommand per job step.
//!
//! ```text
//! unrepresentable-policy classify --head-dir D --base SHA --head SHA [--zizmor-json F] --out F
//! unrepresentable-policy post decide <results-dir> <base-dir>
//! unrepresentable-policy post merge
//! unrepresentable-policy review [--base-dir D] [--head-sha SHA] [PR_NUMBER]
//! unrepresentable-policy push-skip <commit>
//! unrepresentable-policy release-gate <commit>
//! unrepresentable-policy audit <settings.json> [drift-file]
//! unrepresentable-policy differential fetch-public <corpus.lock> <dest>
//! unrepresentable-policy differential fetch-hidden <dest>
//! unrepresentable-policy differential run --base-bin B --head-bin H --prefix P
//!     --base-dir D --base SHA --head SHA --out F
//!     [--head-input name=dir ...] [--input name=dir ...] [--hidden-input name=dir]
//! unrepresentable-policy self-lint --prefix P <base-binary> <head-dir> <base-dir> <base> <head> <out>
//! unrepresentable-policy head-jobs mutants <mutants.out> <exit> <base> <head> <out> <pr.diff>
//! unrepresentable-policy head-jobs checks <status-file> <base> <head> <out>
//! unrepresentable-policy fetch-head <dest-dir> <base-dir>
//! unrepresentable-policy install-tool <tools.lock> <name> <dest-dir>
//! unrepresentable-policy lock-check <Cargo.lock>
//! unrepresentable-policy make-installer --template F --sums F --version V --out F
//! ```

pub(crate) mod args;
mod jobs;
mod policy;

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{run, FIFTH, FIRST, FOURTH, SECOND, SEVENTH, SIXTH, THIRD};

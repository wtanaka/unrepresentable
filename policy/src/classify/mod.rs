//! Classify the paths changed by a pull request. Runs base-branch code only.
//!
//! The head tree is read as data through `git show`, never built or executed.

pub(crate) mod gitdiff;
pub(crate) mod manifest;
pub(crate) mod zizmor;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_git;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{run, Contents, Reader, Request};

#[cfg(test)]
pub(crate) use logic::{classify, Classified, Sides};

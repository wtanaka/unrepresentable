//! Builds the PR comment and the `policy` check run content.
//!
//! Every string that came from the PR goes through `render::render` or
//! `render::render_text`. PR title, body, branch name and commit messages
//! never appear here at all.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{
    build_check, build_comment, not_attributable, Annotation, Approval, Check, Conclusion, Info,
};

#[cfg(test)]
pub(crate) use logic::{annotations, Level};

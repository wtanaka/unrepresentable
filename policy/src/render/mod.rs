//! The one rendering function for every string that came from a pull request.
//!
//! Everything a PR can influence (paths, identifiers, corpus item names,
//! zizmor messages) is rendered through `render`. Nothing else may put such a
//! string into a comment, check title, summary or annotation.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{has_unsafe_chars, render, render_text};

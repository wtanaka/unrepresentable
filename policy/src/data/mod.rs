//! The verdict data stored in the App-authored check run (and handed to the
//! merge step): everything re-evaluation needs, in one JSON object.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{InputResults, Reviewed, VerdictData};

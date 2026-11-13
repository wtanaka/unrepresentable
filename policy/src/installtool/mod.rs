//! Install a pinned tool from `policy/tools.lock` into a directory, verifying
//! its sha256.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{install, Install};

#[cfg(test)]
pub(crate) use logic::parse_lock;

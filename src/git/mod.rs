//! Git access: the change range (union of range diff, working-tree diff and
//! untracked files), and file contents at the base commit.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{
    changes, exists_at, resolve_base, show, with_head, Base, Change, Head, Status,
};

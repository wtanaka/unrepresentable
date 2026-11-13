//! The rule scanner: walks every loaded file with `syn` and applies every
//! rule in every scope.
/// Expression-level rules.
mod exprs;

/// Attribute, signature and item-level rules.
mod items;

/// Macro invocation rules and macro body parsing.
mod macros;

/// Path, method-name and macro-name rules.
mod paths;

/// Pattern rules, such as refutable patterns and wildcards.
mod patterns;

/// Token-level scan used for macro bodies that do not parse as Rust.
mod tokens;

/// Placeholder substitution for `macro_rules!` transcribers.
mod substitute;

/// Type classification helpers: primitives, newtypes and path resolution.
mod types;

/// The `syn` visitor that drives the scan and dispatches to the rule modules.
mod visit;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{
    scan_crate, span_line_col, AssertionSeen, Body, Env, ExpectUse, ImplCtx, Scanner,
};

//! Findings and their deterministic ordering.

use crate::types::{Col, Line, Message, Name, RelPath, Text};

/// A single rule violation at a place in a file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Finding {
    /// Column of the violation.
    pub col: Col,
    /// Enclosing item path, as fingerprints write it.
    pub item: Text,
    /// Line of the violation.
    pub line: Line,
    /// Human-readable description of the violation.
    pub message: Message,
    /// File the violation is in.
    pub path: RelPath,
    /// Name of the rule that was violated.
    pub rule: Name,
}

impl Finding {
    /// Formats the finding as `path:line:col: rule: message`.
    pub(crate) fn render(&self) -> Message {
        Message::from_string(format!(
            "{}:{}:{}: {}: {}",
            self.path.as_str(),
            self.line.get(),
            self.col.get(),
            self.rule.as_str(),
            self.message.as_str()
        ))
    }
}

/// Sorts and removes findings that name the same place, rule and item, even when
/// their messages differ. Findings that sit at one place for a reason that is
/// not a place (line 1 of `Cargo.lock`, of `Cargo.toml`, of a directory) must
/// carry an item that tells them apart, or all but one would be dropped here, and
/// a baseline entry for that one would cover the rest.
pub(crate) fn normalise(mut findings: Vec<Finding>) -> Vec<Finding> {
    findings.sort_by(|left, right| {
        (&left.path, left.line, left.col, &left.rule, &left.message).cmp(&(
            &right.path,
            right.line,
            right.col,
            &right.rule,
            &right.message,
        ))
    });
    findings.dedup_by(|left, right| {
        left.path == right.path
            && left.line == right.line
            && left.col == right.col
            && left.rule == right.rule
            && left.item == right.item
    });
    findings
}

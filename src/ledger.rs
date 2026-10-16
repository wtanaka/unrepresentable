//! The expect ledger and the omission ledger: Markdown tables, first table
//! under a fixed heading.
use crate::config::LEDGER_FILE;

use crate::failure::Failure;

use crate::findings::Finding;

use crate::rules;

use crate::scan::ExpectUse;

use crate::types::{Answer, Col, Count, Holds as _, Line, Message, Name, RelPath, Source, Text};

use crate::util::remove_index;

/// Heading under which the expect ledger table appears.
pub(crate) const EXPECT_HEADING: &str = "## Expect ledger";

/// Heading under which the lint omission table appears.
pub(crate) const OMISSION_HEADING: &str = "## Lint omissions";

/// Name of the file that lists canary exemptions.
pub(crate) const CANARY_EXEMPTIONS_FILE: &str = ".unrepresentable/canary-exemptions.md";

/// The ledger's marker for a lint that exists only on nightly.
pub(crate) const UNSTABLE_MARKER: &str = "unstable";

/// One row of the expect ledger: a justified `#[expect]` attribute.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ExpectRow {
    /// File containing the `#[expect]` attribute.
    pub file: RelPath,
    /// Item the `#[expect]` attribute is attached to.
    pub item: Text,
    /// Line of the ledger row in the ledger file.
    pub line: Line,
    /// Lint the attribute expects.
    pub lint: Text,
    /// The written justification for the expectation.
    pub reason: Text,
}

/// One row of the omission ledger: a lint deliberately left off.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OmissionRow {
    /// What the omission contradicts, or the `-` / `unstable` markers.
    pub contradicts: Text,
    /// Line of the ledger row in the ledger file.
    pub line: Line,
    /// The omitted lint.
    pub lint: Text,
    /// The written justification for the omission.
    pub reason: Text,
}

/// The kind of omission a row claims, from its `Contradicts` column.
pub(crate) enum Omission<'row> {
    /// A lint set to deny that cannot be satisfied together with this one.
    Contradicts(&'row str),
    /// `-`: a preference, printed as a note on every run.
    Preference,
    /// `unstable`: the pinned toolchain rejects the lint as unstable.
    Unstable,
}

impl OmissionRow {
    /// Classifies this row by its `Contradicts` column.
    pub(crate) fn kind(&self) -> Omission<'_> {
        match self.contradicts.as_str().trim() {
            "-" => Omission::Preference,
            UNSTABLE_MARKER => Omission::Unstable,
            other => Omission::Contradicts(other),
        }
    }
}

/// The parsed ledger file: both tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Ledger {
    /// Rows of the expect ledger.
    pub expects: Vec<ExpectRow>,
    /// Rows of the omission ledger.
    pub omissions: Vec<OmissionRow>,
}

impl Ledger {
    /// A ledger with no rows, to be filled as the tables are parsed.
    pub(crate) const fn empty() -> Self {
        Self {
            expects: Vec::new(),
            omissions: Vec::new(),
        }
    }
}

/// One data row of a Markdown table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TableRow {
    /// The trimmed cell texts of the row, with backticks removed.
    pub cells: Vec<Text>,
    /// Line of the row in the source file.
    pub line: Line,
}

/// Rows of the first table under `heading`.
pub(crate) fn table_rows(source: &Source, heading: &Text) -> Result<Vec<TableRow>, Failure> {
    let mut rows = Vec::new();
    let mut in_section = false;
    let mut in_table = false;
    let mut seen_header = false;
    for (index, text) in source.as_str().lines().enumerate() {
        let trimmed = text.trim();
        if trimmed.starts_with("## ") || trimmed.starts_with("# ") {
            if in_table {
                break;
            }
            in_section = trimmed == heading.as_str();
            continue;
        }
        if !in_section {
            continue;
        }
        if !trimmed.starts_with('|') {
            if in_table {
                break;
            }
            continue;
        }
        in_table = true;
        if !seen_header {
            seen_header = true;
            continue;
        }
        let cells: Vec<String> = trimmed
            .trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().replace('`', ""))
            .collect();
        let separator = cells
            .iter()
            .all(|cell| cell.chars().all(|ch| ch == '-' || ch == ':' || ch == ' '));
        let empty = cells
            .iter()
            .all(|cell| cell == "\u{2014}" || cell.is_empty());
        if separator || empty {
            continue;
        }
        rows.push(TableRow {
            cells: cells.into_iter().map(Text::from_string).collect(),
            line: Line::from_index(Count::new(index))?,
        });
    }
    Ok(rows)
}

/// Parses both ledger tables out of the ledger file's `source`.
pub(crate) fn parse(source: &Source) -> Result<Ledger, Failure> {
    let mut ledger = Ledger::empty();
    for row in table_rows(source, &Text::of(EXPECT_HEADING))? {
        let row_line = row.line;
        let [file, item, lint, reason] = row.cells.as_slice() else {
            return Err(Failure::parse_at(
                &RelPath::of(LEDGER_FILE),
                row_line,
                &Message::of("expect ledger rows need columns File | Item | Lint | Reason"),
            ));
        };
        ledger.expects.push(ExpectRow {
            file: RelPath::of(file.as_str()),
            item: item.clone(),
            line: row_line,
            lint: lint.clone(),
            reason: reason.clone(),
        });
    }
    for row in table_rows(source, &Text::of(OMISSION_HEADING))? {
        let row_line = row.line;
        let [lint, contradicts, reason] = row.cells.as_slice() else {
            return Err(Failure::parse_at(
                &RelPath::of(LEDGER_FILE),
                row_line,
                &Message::of("omission ledger rows need columns Lint | Contradicts | Reason"),
            ));
        };
        ledger.omissions.push(OmissionRow {
            contradicts: contradicts.clone(),
            line: row_line,
            lint: lint.clone(),
            reason: reason.clone(),
        });
    }
    Ok(ledger)
}

/// Whether a Reason cell says nothing: empty, or only the placeholder dashes the
/// stub rows use (`-`, an en dash or an em dash).
pub(crate) fn is_blank(reason: &Text) -> Answer {
    matches!(reason.as_str().trim(), "" | "-" | "\u{2013}" | "\u{2014}").answer()
}

/// Multiset match between `#[expect]` uses and ledger rows.
///
/// Uses are deduplicated only when path, item, lint, line and column are all
/// identical, so two uses on different lines of one item need two ledger rows.
pub(crate) fn check_expects(ledger: &Ledger, expects: &[ExpectUse]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut unused: Vec<&ExpectRow> = ledger.expects.iter().collect();
    for used in expects {
        let position = unused.iter().position(|row| {
            (&row.file, &row.item, &row.lint) == (&used.path, &used.item, &used.lint)
        });
        match position {
            Some(found) => remove_index(&mut unused, Count::new(found)),
            None => findings.push(Finding {
                col: used.col,
                item: used.item.clone(),
                line: used.line,
                message: Message::from_string(format!(
                    "`#[expect({})]` has no row in {LEDGER_FILE} for item `{}`",
                    used.lint.as_str(),
                    used.item.as_str()
                )),
                path: used.path.clone(),
                rule: Name::of(rules::SUPPRESSION.name()),
            }),
        }
    }
    for row in unused {
        findings.push(ledger_finding(
            row.line,
            &Message::of("stale expect-ledger row: no matching #[expect] attribute"),
        ));
    }
    for row in &ledger.expects {
        if is_blank(&row.reason) == Answer::Yes {
            findings.push(ledger_finding(
                row.line,
                &Message::of("expect-ledger row has an empty Reason"),
            ));
        }
    }
    findings
}

/// Builds a finding about the ledger file itself at `line`.
fn ledger_finding(line: Line, message: &Message) -> Finding {
    Finding {
        col: Col::new(1),
        item: Text::from_string(String::new()),
        line,
        message: message.clone(),
        path: RelPath::of(LEDGER_FILE),
        rule: Name::of(rules::SUPPRESSION.name()),
    }
}

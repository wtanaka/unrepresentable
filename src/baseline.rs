//! Fingerprints, the two-way baseline check, and `check --ratchet`.
use alloc::collections::BTreeMap;

use crate::config::{BaselineEntry, BaselineMode, Config, CONFIG_FILE};

use crate::failure::Failure;

use crate::findings::Finding;

use crate::rules;

use crate::types::{Col, Count, Line, Message, Name, RelPath, Source, Text};

use crate::util::remove_index;

/// A stable identity for a finding: file, rule, item and occurrence index.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Fingerprint {
    /// The file the finding is in.
    pub file: RelPath,
    /// Which occurrence of the same rule, file and item this is, from zero.
    pub index: Count,
    /// The item the finding is attached to.
    pub item: Text,
    /// The id of the rule that fired.
    pub rule: Name,
}

impl Fingerprint {
    /// The fingerprint a baseline entry records.
    fn of_entry(entry: &BaselineEntry) -> Self {
        Self {
            file: entry.file.clone(),
            index: entry.index,
            item: entry.item.clone(),
            rule: entry.rule.clone(),
        }
    }
}

/// A fingerprint and the position of its finding in the sorted list.
#[derive(Clone, Debug)]
pub(crate) struct Positioned {
    /// Index of the finding in the sorted findings list.
    pub position: Count,
    /// The finding's fingerprint.
    pub print: Fingerprint,
}

/// What a fingerprint group is counted by: rule, then crate.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Group {
    /// The name of the crate the entries belong to.
    pub member: Name,
    /// The rule id the entries are for.
    pub rule: Name,
}

/// The result of matching findings against the baseline.
pub(crate) struct Applied {
    /// Stale entries and forbidden-mode findings.
    pub baseline_findings: Vec<Finding>,
    /// Fingerprints of findings that a baseline entry covers.
    pub matched: Vec<Fingerprint>,
    /// Findings that remain after the baseline.
    pub remaining: Vec<Finding>,
    /// Baseline entries that match no finding.
    pub stale: Vec<BaselineEntry>,
    /// Fingerprints of findings that no baseline entry covers.
    pub unbaselined: Vec<Positioned>,
}

/// A member directory and the member's name.
#[derive(Clone, Debug)]
pub(crate) struct MemberDir {
    /// The member's directory relative to the workspace root.
    pub dir: RelPath,
    /// The member's crate name.
    pub name: Name,
}

/// Fingerprints of `findings` (already sorted by position), with occurrence indices.
pub(crate) fn fingerprints(findings: &[Finding]) -> Result<Vec<Positioned>, Failure> {
    let mut counts: BTreeMap<(Name, RelPath, Text), Count> = BTreeMap::new();
    let mut out = Vec::new();
    for (position, finding) in findings.iter().enumerate() {
        let key = (
            finding.rule.clone(),
            finding.path.clone(),
            finding.item.clone(),
        );
        let counter = match counts.get(&key) {
            Some(existing) => *existing,
            None => Count::new(0),
        };
        counts.extend([(key, counter.next()?)]);
        out.push(Positioned {
            position: Count::new(position),
            print: Fingerprint {
                file: finding.path.clone(),
                index: counter,
                item: finding.item.clone(),
                rule: finding.rule.clone(),
            },
        });
    }
    Ok(out)
}

/// A baseline-rule finding located at `line` of the config file.
fn config_finding(line: Line, message: &Message) -> Finding {
    Finding {
        col: Col::new(1),
        item: Text::from_string(String::new()),
        line,
        message: message.clone(),
        path: RelPath::of(CONFIG_FILE),
        rule: Name::of(rules::BASELINE.name()),
    }
}

/// Matches findings against the baseline, reporting stale entries and forbidden mode.
pub(crate) fn apply(config: &Config, findings: &[Finding]) -> Result<Applied, Failure> {
    let prints = fingerprints(findings)?;
    let mut baseline_findings = Vec::new();
    match config.baseline_mode {
        BaselineMode::Forbidden => {
            for entry in &config.baseline {
                baseline_findings.push(config_finding(
                    entry.line,
                    &Message::of("baseline entries are forbidden (baseline = \"forbidden\")"),
                ));
            }
        }
        BaselineMode::ShrinkOnly => {}
    }
    let mut remaining_entries: Vec<&BaselineEntry> = config.baseline.iter().collect();
    let mut matched = Vec::new();
    let mut unbaselined = Vec::new();
    let mut remaining = Vec::new();
    for positioned in prints {
        let found = remaining_entries
            .iter()
            .position(|entry| Fingerprint::of_entry(entry) == positioned.print);
        if let Some(slot) = found {
            remove_index(&mut remaining_entries, Count::new(slot));
            matched.push(positioned.print);
        } else {
            if let Some(finding) = findings.get(positioned.position.get()) {
                remaining.push(finding.clone());
            }
            unbaselined.push(positioned);
        }
    }
    let stale: Vec<BaselineEntry> = remaining_entries.into_iter().cloned().collect();
    for entry in &stale {
        baseline_findings.push(config_finding(
            entry.line,
            &Message::of("stale baseline entry: no matching finding; remove it"),
        ));
    }
    Ok(Applied {
        baseline_findings,
        matched,
        remaining,
        stale,
        unbaselined,
    })
}

/// The member a file belongs to: the longest member directory prefix. `--ratchet`
/// keys its re-key rule per (rule, crate) with this.
pub(crate) fn crate_of(members: &[MemberDir], file: &RelPath) -> Name {
    let mut best: Option<&MemberDir> = None;
    for candidate in members {
        let dir = candidate.dir.as_str();
        let inside =
            dir.is_empty() || file.as_str() == dir || file.as_str().starts_with(&format!("{dir}/"));
        let longer = match best {
            Some(current) => candidate.dir.as_str().len() > current.dir.as_str().len(),
            None => true,
        };
        if inside && longer {
            best = Some(candidate);
        }
    }
    match best {
        Some(found) => found.name.clone(),
        None => Name::of(""),
    }
}

/// The entries `--ratchet` would write: drops stale entries and re-keys
/// within a (rule, crate) group only when its count does not rise.
pub(crate) fn ratchet_entries(
    applied: &Applied,
    members: &[MemberDir],
) -> Result<Vec<Fingerprint>, Failure> {
    let mut entries: Vec<Fingerprint> = applied.matched.clone();
    let mut stale_counts: BTreeMap<Group, Count> = BTreeMap::new();
    for entry in &applied.stale {
        let group = Group {
            member: crate_of(members, &entry.file),
            rule: entry.rule.clone(),
        };
        let current = match stale_counts.get(&group) {
            Some(existing) => *existing,
            None => Count::new(0),
        };
        stale_counts.extend([(group, current.next()?)]);
    }
    let mut new_by_group: BTreeMap<Group, Vec<&Fingerprint>> = BTreeMap::new();
    for positioned in &applied.unbaselined {
        let group = Group {
            member: crate_of(members, &positioned.print.file),
            rule: positioned.print.rule.clone(),
        };
        new_by_group
            .entry(group)
            .or_default()
            .push(&positioned.print);
    }
    for (group, prints) in new_by_group {
        let stale = match stale_counts.get(&group) {
            Some(count) => count.get(),
            None => 0,
        };
        if prints.len() <= stale {
            entries.extend(prints.into_iter().cloned());
        }
    }
    entries.sort();
    Ok(entries)
}

/// Renders the baseline blocks.
pub(crate) fn render(entries: &[Fingerprint]) -> Source {
    let mut out = String::new();
    for entry in entries {
        out.push_str("[[baseline-entry]]\n");
        out.extend([format!(
            "rule = {}\n",
            quote(&Text::of(entry.rule.as_str())).as_str()
        )]);
        out.extend([format!(
            "file = {}\n",
            quote(&Text::of(entry.file.as_str())).as_str()
        )]);
        out.extend([format!("item = {}\n", quote(&entry.item).as_str())]);
        out.extend([format!("index = {}\n\n", entry.index.get())]);
    }
    Source::from_string(out)
}

/// Quotes `text` as a TOML basic string, escaping quotes, backslashes and newlines.
fn quote(text: &Text) -> Text {
    let mut out = String::from("\"");
    for ch in text.as_str().chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out.push('"');
    Text::from_string(out)
}

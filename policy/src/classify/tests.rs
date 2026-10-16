use super::gitdiff::{parse_name_status, parse_raw_modes, Entry, Status};
use super::manifest::lock_is_registry_only;
use super::zizmor;
use super::{classify, Classified, Contents, Reader, Sides};
use crate::bytes::Bytes;
use crate::failure::Failure;
use crate::reason::{Code, Reason};
use crate::testkit::Fail;
use crate::types::{Answer, Count, Name, RelPath, Text};
use alloc::collections::{BTreeMap, BTreeSet};

macro_rules! tree {
    ($(($path:expr_2021, $text:expr_2021)),* $(,)?) => {
        Tree(BTreeMap::from([$((RelPath::of($path), Text::of($text))),*]))
    };
}

macro_rules! changed {
    ($(($status:ident, $path:expr_2021)),* $(,)?) => {
        vec![$(Entry { status: Status::$status, path: RelPath::of($path) }),*]
    };
}

macro_rules! count_of {
    ($classified:expr_2021, $key:literal) => {
        $classified.counts.get(&Name::of($key))
    };
}

macro_rules! manifest_change {
    ($base:expr_2021, $head:expr_2021 $(,)?) => {
        Case::of(changed![(Modified, "Cargo.toml")])
            .with_base(tree!(("Cargo.toml", $base)))
            .with_head(tree!(("Cargo.toml", $head)))
    };
}

macro_rules! lock_with {
    ($version:literal) => {
        LOCK.replace("VERSION", $version)
    };
}

const LOCK: &str = "[[package]]\nname = \"syn\"\nversion = \"VERSION\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"aa\"\n";

/// A commit's files, by path.
struct Tree(BTreeMap<RelPath, Text>);

impl Reader for Tree {
    fn read(&self, path: &RelPath) -> Contents {
        match self.0.get(path) {
            Some(found) => Contents::Present(found.clone()),
            None => Contents::Absent,
        }
    }
}

/// A change to classify: entries, both trees and the symlinks.
struct Case {
    bad: BTreeSet<RelPath>,
    base: Tree,
    entries: Vec<Entry>,
    head: Tree,
}

impl Case {
    fn of(entries: Vec<Entry>) -> Self {
        Self {
            bad: BTreeSet::new(),
            base: tree!(),
            entries,
            head: tree!(),
        }
    }

    fn run(&self) -> Result<Classified, Failure> {
        classify(
            &self.entries,
            &self.bad,
            &Sides {
                base: &self.base,
                head: &self.head,
            },
        )
    }

    fn with_bad(self, path: &RelPath) -> Self {
        Self {
            bad: BTreeSet::from([path.clone()]),
            ..self
        }
    }

    fn with_base(self, base: Tree) -> Self {
        Self { base, ..self }
    }

    fn with_head(self, head: Tree) -> Self {
        Self { head, ..self }
    }
}

fn codes(reasons: &[Reason]) -> Vec<Code> {
    reasons.iter().map(|reason| reason.code).collect()
}

#[test]
fn docs_are_auto_and_source_is_not() {
    assert_eq!(
        check_docs_are_auto_and_source_is_not(),
        Ok(()),
        "docs_are_auto_and_source_is_not"
    );
}

fn check_docs_are_auto_and_source_is_not() -> Result<(), Fail> {
    let docs = Case::of(changed![(Modified, "README.md"), (Modified, "docs/x.md")]).run()?;
    ensure_eq!(&docs.reasons, &Vec::<Reason>::new(), "no reasons for docs");
    ensure_eq!(count_of!(docs, "auto"), Count::new(2), "two auto files");
    let source = Case::of(changed![
        (Modified, "src/rules.rs"),
        (Added, "src/scan/new.rs"),
        (Modified, "src/lintcov.rs"),
        (Modified, "src/against.rs"),
        (Added, "src/data.txt"),
        (Modified, "SRC/Scan/Mixed.rs"),
    ])
    .run()?;
    ensure_eq!(
        codes(&source.reasons),
        vec![Code::RulesCodeChanged; 6],
        "every file under src/ is rules code, whatever its name, case or depth"
    );
    ensure_eq!(count_of!(source, "auto"), Count::new(0), "none is auto");
    ensure_eq!(
        count_of!(source, "human"),
        Count::new(6),
        "all need a human"
    );
    let elsewhere = Case::of(changed![
        (Modified, "policy/src/main.rs"),
        (Added, "canaries/rust/src/bin/c.rs"),
        (Added, "tests/fixtures/src/a.rs"),
    ])
    .run()?;
    ensure_eq!(
        codes(&elsewhere.reasons),
        vec![Code::PolicyChanged],
        "a nested src directory is not the tool's src/: policy keeps its own code, append-only stays append-only"
    );
    Ok(())
}

#[test]
fn new_fixture_is_append_only_not_a_reason() {
    assert_eq!(
        check_new_fixture_is_append_only_not_a_reason(),
        Ok(()),
        "new_fixture_is_append_only_not_a_reason"
    );
}

fn check_new_fixture_is_append_only_not_a_reason() -> Result<(), Fail> {
    let done = Case::of(changed![
        (Added, "tests/fixtures/x.rs"),
        (Added, "tests/expected/x.txt"),
        (Added, "tests/canaries/c.rs"),
        (Added, "canaries/rust/src/bin/c.rs"),
    ])
    .run()?;
    ensure_eq!(&done.reasons, &Vec::<Reason>::new(), "no reasons");
    ensure_eq!(
        count_of!(done, "append_only"),
        Count::new(4),
        "four append-only files"
    );
    Ok(())
}

#[test]
fn modified_or_deleted_fixture_is_judge_modified() {
    assert_eq!(
        check_modified_or_deleted_fixture_is_judge_modified(),
        Ok(()),
        "modified_or_deleted_fixture_is_judge_modified"
    );
}

fn check_modified_or_deleted_fixture_is_judge_modified() -> Result<(), Fail> {
    let done = Case::of(changed![
        (Modified, "tests/fixtures/x.rs"),
        (Deleted, "tests/expected/y.txt"),
    ])
    .run()?;
    ensure_eq!(
        codes(&done.reasons),
        vec![Code::JudgeModified, Code::JudgeModified],
        "both are judge changes"
    );
    let first = done.reasons.first().map(|reason| reason.evidence.clone());
    ensure_eq!(
        first,
        Some(Text::of(
            "tests/fixtures/x.rs (append-only directory, modified)"
        )),
        "evidence says how"
    );
    let second = done.reasons.get(1).map(|reason| reason.evidence.clone());
    ensure_eq!(
        second,
        Some(Text::of(
            "tests/expected/y.txt (append-only directory, deleted)"
        )),
        "evidence says deleted"
    );
    Ok(())
}

#[test]
fn protected_paths_are_human() {
    assert_eq!(
        check_protected_paths_are_human(),
        Ok(()),
        "protected_paths_are_human"
    );
}

fn check_protected_paths_are_human() -> Result<(), Fail> {
    let paths = [
        ".github/workflows/ci.yml",
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "clippy.toml",
        "deny.toml",
        ".deny.toml",
        "crates/a/deny.toml",
        ".unrepresentable/config",
        ".unrepresentable/ledger.md",
        ".unrepresentable/canary-exemptions.md",
        ".unrepresentable/notes.md",
        "policy/README.md",
        "policy/src/main.rs",
        "ci/unrepresentable.sh",
        ".github/CODEOWNERS",
        "CODEOWNERS",
        "LICENSE",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "SECURITY.md",
        "tests/policy/test_x.py",
        "build.rs",
        ".cargo/config.toml",
        "tests/foo.rs",
    ];
    for path in paths {
        let done = Case::of(changed![(Modified, path)]).run()?;
        ensure!(!done.reasons.is_empty(), "{path} needs a reason");
        ensure_eq!(count_of!(done, "human"), Count::new(1), "{path} is human");
    }
    Ok(())
}

#[test]
fn workflow_code() {
    assert_eq!(check_workflow_code(), Ok(()), "workflow_code");
}

fn check_workflow_code() -> Result<(), Fail> {
    let done = Case::of(changed![(Modified, ".github/workflows/ci.yml")]).run()?;
    ensure_eq!(
        codes(&done.reasons),
        vec![Code::WorkflowChanged],
        "workflow code"
    );
    let evidence = done.reasons.first().map(|reason| reason.evidence.clone());
    ensure_eq!(
        evidence,
        Some(Text::of(".github/workflows/ci.yml")),
        "evidence is the path"
    );
    Ok(())
}

#[test]
fn tool_directory_codes() {
    assert_eq!(check_tool_directory_codes(), Ok(()), "tool_directory_codes");
}

fn check_tool_directory_codes() -> Result<(), Fail> {
    let expected = [
        (".unrepresentable/config", Code::ConfigFileChanged),
        (".unrepresentable/ledger.md", Code::LedgerFileChanged),
        (
            ".unrepresentable/canary-exemptions.md",
            Code::LedgerFileChanged,
        ),
        (".unrepresentable/notes.md", Code::LedgerFileChanged),
    ];
    for (path, code) in expected {
        let done = Case::of(changed![(Modified, path)]).run()?;
        ensure_eq!(codes(&done.reasons), vec![code], "{path} code");
    }
    Ok(())
}

#[test]
fn case_tricks_do_not_escape_protection() {
    assert_eq!(
        check_case_tricks_do_not_escape_protection(),
        Ok(()),
        "case_tricks_do_not_escape_protection"
    );
}

fn check_case_tricks_do_not_escape_protection() -> Result<(), Fail> {
    for path in [
        ".GitHub/Workflows/x.yml",
        "CARGO.TOML",
        "Policy/render.py",
        "Unrepresentable.CFG",
        "Policy-RS/src/main.rs",
    ] {
        let done = Case::of(changed![(Added, path)]).run()?;
        ensure!(!done.reasons.is_empty(), "{path} must not slip through");
    }
    Ok(())
}

#[test]
fn unknown_path_is_human_by_default() {
    assert_eq!(
        check_unknown_path_is_human_by_default(),
        Ok(()),
        "unknown_path_is_human_by_default"
    );
}

fn check_unknown_path_is_human_by_default() -> Result<(), Fail> {
    let done = Case::of(changed![(Added, "scripts/deploy.sh")]).run()?;
    ensure_eq!(
        codes(&done.reasons),
        vec![Code::OtherPathChanged],
        "default deny"
    );
    Ok(())
}

#[test]
fn unsafe_path_chars() {
    assert_eq!(check_unsafe_path_chars(), Ok(()), "unsafe_path_chars");
}

fn check_unsafe_path_chars() -> Result<(), Fail> {
    for path in [
        "src/a\nb.rs",
        "src/a\u{202e}b.rs",
        "src/a\u{200b}b.rs",
        "src/a\u{0}b.rs",
        "src/a\u{85}b.rs",
        "src/\u{feff}a.rs",
    ] {
        let done = Case::of(changed![(Added, path)]).run()?;
        ensure_eq!(
            codes(&done.reasons),
            vec![Code::UnsafePathChars],
            "{path:?}"
        );
        let first = done.reasons.first();
        ensure_eq!(
            first.map(|reason| reason.path.clone()),
            Some(None),
            "no path"
        );
        ensure!(
            first.is_some_and(|reason| !reason.evidence.as_str().contains('\n')),
            "evidence has no raw newline"
        );
        ensure_eq!(count_of!(done, "human"), Count::new(1), "counted as human");
    }
    Ok(())
}

#[test]
fn unsafe_check_precedes_auto() {
    assert_eq!(
        check_unsafe_check_precedes_auto(),
        Ok(()),
        "unsafe_check_precedes_auto"
    );
}

fn check_unsafe_check_precedes_auto() -> Result<(), Fail> {
    let done = Case::of(changed![(Added, "src/x\n\u{2705} ok.rs")]).run()?;
    ensure_eq!(
        codes(&done.reasons),
        vec![Code::UnsafePathChars],
        "unsafe wins"
    );
    Ok(())
}

#[test]
fn symlink_is_human() {
    assert_eq!(check_symlink_is_human(), Ok(()), "symlink_is_human");
}

fn check_symlink_is_human() -> Result<(), Fail> {
    let done = Case::of(changed![(Added, "src/link.rs")])
        .with_bad(&RelPath::of("src/link.rs"))
        .run()?;
    ensure_eq!(
        codes(&done.reasons),
        vec![Code::SymlinkOrSubmodule],
        "symlinks are human"
    );
    Ok(())
}

#[test]
fn lint_lowered_with_evidence() {
    assert_eq!(
        check_lint_lowered_with_evidence(),
        Ok(()),
        "lint_lowered_with_evidence"
    );
}

fn check_lint_lowered_with_evidence() -> Result<(), Fail> {
    let done = manifest_change!(
        "[workspace.lints.clippy]\nunwrap_used = \"deny\"\nexpect_used = \"deny\"\n",
        "[workspace.lints.clippy]\nexpect_used = \"deny\"\n\nunwrap_used = \"warn\"\n",
    )
    .run()?;
    let lowered: Vec<&Reason> = done
        .reasons
        .iter()
        .filter(|reason| reason.code == Code::LintLowered)
        .collect();
    ensure_eq!(lowered.len(), 1, "one lint was lowered");
    ensure_eq!(
        lowered.first().map(|reason| reason.evidence.clone()),
        Some(Text::of(
            "clippy::unwrap_used deny\u{2192}warn (Cargo.toml:4)"
        )),
        "evidence names lint, levels and line"
    );
    ensure!(
        codes(&done.reasons).contains(&Code::ManifestChanged),
        "the manifest itself is a reason too"
    );
    Ok(())
}

#[test]
fn lint_removed_and_inline_table_levels() {
    assert_eq!(
        check_lint_removed_and_inline_table_levels(),
        Ok(()),
        "lint_removed_and_inline_table_levels"
    );
}

fn check_lint_removed_and_inline_table_levels() -> Result<(), Fail> {
    let done = manifest_change!(
        "[workspace.lints.rust]\nunsafe_code = { level = \"forbid\", priority = 1 }\ndead_code = \"deny\"\n",
        "[workspace.lints.rust]\nunsafe_code = { level = \"deny\", priority = 1 }\n",
    )
    .run()?;
    let evidence: Vec<String> = done
        .reasons
        .iter()
        .filter(|reason| reason.code == Code::LintLowered)
        .map(|reason| reason.evidence.as_str().to_owned())
        .collect();
    ensure_eq!(evidence.len(), 2, "two lints weakened");
    ensure!(
        evidence
            .iter()
            .any(|text| text.starts_with("unsafe_code forbid\u{2192}deny")),
        "forbid to deny is reported: {evidence:?}"
    );
    ensure!(
        evidence
            .iter()
            .any(|text| text.starts_with("dead_code deny\u{2192}(removed)")),
        "removal is reported: {evidence:?}"
    );
    Ok(())
}

#[test]
fn lint_raised_is_not_lowered() {
    assert_eq!(
        check_lint_raised_is_not_lowered(),
        Ok(()),
        "lint_raised_is_not_lowered"
    );
}

fn check_lint_raised_is_not_lowered() -> Result<(), Fail> {
    let done = manifest_change!(
        "[lints.clippy]\nx = \"warn\"\n",
        "[lints.clippy]\nx = \"deny\"\n",
    )
    .run()?;
    ensure!(
        !codes(&done.reasons).contains(&Code::LintLowered),
        "raising a lint is fine"
    );
    Ok(())
}

#[test]
fn lint_lowering_is_seen_however_the_tables_are_spelled() {
    assert_eq!(
        check_lint_lowering_is_seen_however_the_tables_are_spelled(),
        Ok(()),
        "lint_lowering_is_seen_however_the_tables_are_spelled"
    );
}

fn check_lint_lowering_is_seen_however_the_tables_are_spelled() -> Result<(), Fail> {
    let done = manifest_change!(
        "[workspace.lints.clippy]\nx = { level = \"deny\", priority = -1 }\n\n[lints]\nrust.y = \"forbid\"\n",
        "[workspace]\nlints = { clippy = { x = \"allow\" } }\n\n[lints.rust]\ny = \"warn\"\n",
    )
    .run()?;
    let evidence: Vec<Text> = done
        .reasons
        .iter()
        .filter(|reason| reason.code == Code::LintLowered)
        .map(|reason| reason.evidence.clone())
        .collect();
    ensure_eq!(
        evidence,
        vec![
            Text::of("clippy::x deny\u{2192}allow (Cargo.toml:2)"),
            Text::of("y forbid\u{2192}warn (Cargo.toml:5)"),
        ],
        "dotted keys, inline tables and headers all count"
    );
    Ok(())
}

#[test]
fn an_unreadable_manifest_or_lock_is_an_error_not_an_empty_result() {
    assert_eq!(
        check_an_unreadable_manifest_or_lock_is_an_error(),
        Ok(()),
        "an_unreadable_manifest_or_lock_is_an_error_not_an_empty_result"
    );
}

fn check_an_unreadable_manifest_or_lock_is_an_error() -> Result<(), Fail> {
    let broken_manifest = manifest_change!(
        "[lints.clippy]\nx = \"deny\"\n",
        "[lints.clippy\nx = \"allow\"\n",
    )
    .run();
    ensure!(
        broken_manifest.is_err(),
        "a head manifest that is not TOML must not read as having no lints"
    );
    let broken_lock = Case::of(changed![(Modified, "Cargo.lock")])
        .with_base(tree!(("Cargo.lock", &lock_with!("2.0.119"))))
        .with_head(tree!(("Cargo.lock", "[[package]\nname = \"syn\"\n")))
        .run();
    ensure!(
        broken_lock.is_err(),
        "a head lock file that is not TOML is an error"
    );
    ensure!(
        lock_is_registry_only(&Text::of("[[package]\n")).is_err(),
        "the registry-only check refuses a lock it cannot read"
    );
    Ok(())
}

#[test]
fn lock_fields_are_read_whatever_the_spacing_and_order() {
    assert_eq!(
        check_lock_fields_are_read_whatever_the_spacing_and_order(),
        Ok(()),
        "lock_fields_are_read_whatever_the_spacing_and_order"
    );
}

fn check_lock_fields_are_read_whatever_the_spacing_and_order() -> Result<(), Fail> {
    let reordered = "version = 4\n\n[[package]]\nchecksum = 'aa'\nversion='1'\nsource =\"registry+https://github.com/rust-lang/crates.io-index\" # note\nname = \"x\"\n\n[[package]]\nname = \"local\"\nversion = \"0.1.0\"\n";
    ensure_eq!(
        lock_is_registry_only(&Text::of(reordered)),
        Ok(Answer::Yes),
        "field order, quote style and comments do not matter"
    );
    Ok(())
}

#[test]
fn dropping_workspace_inheritance_is_lowering() {
    assert_eq!(
        check_dropping_workspace_inheritance_is_lowering(),
        Ok(()),
        "dropping_workspace_inheritance_is_lowering"
    );
}

fn check_dropping_workspace_inheritance_is_lowering() -> Result<(), Fail> {
    let done = Case::of(changed![(Modified, "crates/a/Cargo.toml")])
        .with_base(tree!((
            "crates/a/Cargo.toml",
            "[lints]\nworkspace = true\n"
        )))
        .with_head(tree!(("crates/a/Cargo.toml", "[package]\nname = \"a\"\n")))
        .run()?;
    ensure!(
        codes(&done.reasons).contains(&Code::LintLowered),
        "inheritance removal lowers lints"
    );
    Ok(())
}

#[test]
fn dependency_bump() {
    assert_eq!(check_dependency_bump(), Ok(()), "dependency_bump");
}

fn check_dependency_bump() -> Result<(), Fail> {
    let done = Case::of(changed![(Modified, "Cargo.lock")])
        .with_base(tree!(("Cargo.lock", &lock_with!("2.0.119"))))
        .with_head(tree!(("Cargo.lock", &lock_with!("2.0.120"))))
        .run()?;
    let evidence: Vec<Text> = done
        .reasons
        .iter()
        .filter(|reason| reason.code == Code::DependencyChanged)
        .map(|reason| reason.evidence.clone())
        .collect();
    ensure_eq!(
        evidence,
        vec![Text::of("Cargo.lock syn 2.0.119\u{2192}2.0.120")],
        "version bump evidence"
    );
    Ok(())
}

#[test]
fn dependency_added() {
    assert_eq!(check_dependency_added(), Ok(()), "dependency_added");
}

fn check_dependency_added() -> Result<(), Fail> {
    let head = format!(
        "{}\n[[package]]\nname = \"evil\"\nversion = \"0.1.0\"\n",
        lock_with!("2.0.119")
    );
    let done = Case::of(changed![(Modified, "Cargo.lock")])
        .with_base(tree!(("Cargo.lock", &lock_with!("2.0.119"))))
        .with_head(tree!(("Cargo.lock", &head)))
        .run()?;
    let evidence: Vec<Text> = done
        .reasons
        .iter()
        .filter(|reason| reason.code == Code::DependencyChanged)
        .map(|reason| reason.evidence.clone())
        .collect();
    ensure_eq!(
        evidence,
        vec![Text::of("Cargo.lock evil (added 0.1.0)")],
        "added package evidence"
    );
    Ok(())
}

#[test]
fn lock_registry_only() {
    assert_eq!(
        lock_is_registry_only(&Text::from_string(lock_with!("1"))),
        Ok(Answer::Yes),
        "crates.io with a checksum is fine"
    );
    let git =
        "[[package]]\nname = \"x\"\nversion = \"1\"\nsource = \"git+https://example.com/x\"\n";
    assert_eq!(
        lock_is_registry_only(&Text::of(git)),
        Ok(Answer::No),
        "git sources are not"
    );
    let unsummed = "[[package]]\nname=\"x\"\nversion=\"1\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n";
    assert_eq!(
        lock_is_registry_only(&Text::of(unsummed)),
        Ok(Answer::No),
        "a missing checksum is not"
    );
}

#[test]
fn name_status_parser() {
    assert_eq!(
        parse_name_status(&Bytes::of(b"M\0src/a.rs\0A\0src/b\nc.rs\0")),
        Ok(vec![
            Entry {
                path: RelPath::of("src/a.rs"),
                status: Status::Modified,
            },
            Entry {
                path: RelPath::of("src/b\nc.rs"),
                status: Status::Added,
            },
        ]),
        "two records"
    );
    assert_eq!(
        parse_name_status(&Bytes::of(b"")),
        Ok(Vec::new()),
        "empty diff"
    );
    for bad in [&b"M\0src/a.rs"[..], b"M\0", b"Z\0x\0", b"R100\0a\0b\0"] {
        assert!(
            parse_name_status(&Bytes::of(bad)).is_err(),
            "malformed output is refused: {bad:?}"
        );
    }
}

#[test]
fn raw_modes() {
    let raw = b":000000 120000 0000000 abcdef0 A\0src/link.rs\0:100644 100644 a b M\0src/ok.rs\0";
    assert_eq!(
        parse_raw_modes(&Bytes::of(raw)),
        Ok(BTreeSet::from([RelPath::of("src/link.rs")])),
        "only the symlink is listed"
    );
}

#[test]
fn zizmor_reasons() {
    let json = "[{\"ident\":\"template-injection\",\"desc\":\"bad `x`\",\"locations\":[{\"symbolic\":{\"key\":{\"Local\":{\"given_path\":\"workflows/a.yml\"}}},\"concrete\":{\"location\":{\"start_point\":{\"row\":4}}}}]}]";
    let found = zizmor::reasons(&Text::of(json));
    assert_eq!(
        found,
        Ok(vec![Reason::at(
            Code::ZizmorFinding,
            &Text::of("template-injection: bad `x` (.github/workflows/a.yml:5)"),
            &RelPath::of(".github/workflows/a.yml"),
            Count::new(5),
        )]),
        "path is under .github and the row is one based"
    );
    assert_eq!(
        zizmor::reasons(&Text::of("[]")),
        Ok(Vec::new()),
        "an empty list is no findings"
    );
}

#[test]
fn empty_or_invalid_zizmor_output_is_an_error_not_no_findings() {
    for text in [
        "",
        "   \n",
        "not json",
        "{}",
        "null",
        "{\"findings\":[]}",
        "[{",
    ] {
        assert!(
            zizmor::reasons(&Text::of(text)).is_err(),
            "{text:?} is an error, which fails the classify job"
        );
    }
}

use super::corpus::{check_listing, parse_lock, parse_lock_line, safe_extract, Listing, Unpack};
use super::workspace::make_workspace;
use super::{
    compare, hidden_reasons, lost, parse_output, rule_source, to_reasons, FindingKey, Findings,
    Input, LostFinding, Origin, TargetDirs, Tools,
};
use crate::bytes::Bytes;
use crate::io;
use crate::process::{self, Args};
use crate::reason::Code;
use crate::testkit::{Fail, Scratch};
use crate::types::{Answer, Count, Glyph, Message, Name, RelPath, Text};
use alloc::collections::BTreeSet;
use core::slice;
use std::path::{Path, PathBuf};

macro_rules! key {
    ($path:literal, $rule:literal, $message:literal) => {
        FindingKey {
            path: Text::of($path),
            rule: Text::of($rule),
            message: Text::of($message),
        }
    };
}

/// A script that prints the given lines and exits 1.
macro_rules! printing {
    ($scratch:expr_2021, $name:literal, [$($line:literal),* $(,)?]) => {
        write_script(
            &$scratch,
            &Name::of($name),
            &Text::from_string(format!(
                "{}exit 1\n",
                [$($line),*]
                    .iter()
                    .map(|line| format!("echo '{line}'\n"))
                    .collect::<Vec<String>>()
                    .concat()
            )),
        )
    };
}

macro_rules! script {
    ($scratch:expr_2021, $name:literal, $body:literal) => {
        write_script(&$scratch, &Name::of($name), &Text::of($body))
    };
}

/// Files of a pull request that would make cargo, rustup or the tool run code.
const HOSTILE_TREE: [(&str, &str); 11] = [
    (".unrepresentable/config", "baseline = \"forbidden\"\n"),
    (
        ".cargo/config.toml",
        "[build]\nrustc-wrapper = \"/tmp/evil\"\n",
    ),
    (
        "rust-toolchain.toml",
        "[toolchain]\nchannel = \"/tmp/evil\"\n",
    ),
    ("build.rs", "fn main() {}\n"),
    (
        "Cargo.toml",
        "[package]\nname = \"evil\"\nbuild = \"b.rs\"\n",
    ),
    ("Cargo.lock", "evil lock\n"),
    ("a.rs", "fn a() {}\n"),
    ("deep/.cargo/config", "evil\n"),
    ("deep/build.rs", "fn main() {}\n"),
    ("deep/rust-toolchain", "evil\n"),
    ("deep/b.rs", "fn b() {}\n"),
];

/// A script that exits 2 unless the workspace is complete and it may build,
/// else reports one finding and exits 1.
const NEEDS_WORKSPACE: &str = "test \"$1\" = check || exit 2\n\
    test \"$2\" = --range || exit 2\n\
    test \"$3\" = HEAD || exit 2\n\
    root=\"$4\"\n\
    test -f \"$root/.unrepresentable/config\" || exit 2\n\
    test -f \"$root/.unrepresentable/ledger.md\" || exit 2\n\
    test -f \"$root/Cargo.lock\" || exit 2\n\
    test \"$(git -C \"$root\" rev-list --count HEAD)\" = 1 || exit 2\n\
    test -n \"$CARGO_TARGET_DIR\" || exit 2\n\
    touch \"$CARGO_TARGET_DIR/probe\" || exit 2\n\
    echo \"src/a.rs:1:1: r1: m1\"\n\
    exit 1\n";

#[test]
fn parse_findings_ignores_line_numbers() {
    let output =
        Text::of("src/a.rs:3:1: no-unwrap: unwrap on Option\nnoise\nsrc/b.rs:9:2: rule-x: msg\n");
    let parsed = parse_output(&output);
    assert_eq!(
        parsed.as_ref().map(|found| found.findings.clone()),
        Ok(BTreeSet::from([
            key!("src/a.rs", "no-unwrap", "unwrap on Option"),
            key!("src/b.rs", "rule-x", "msg"),
        ])),
        "positions are dropped"
    );
    assert_eq!(
        parsed.map(|found| found.unparsed),
        Ok(Count::new(1)),
        "a line that is not a finding is counted"
    );
}

#[test]
fn only_lost_findings_count() {
    let base = BTreeSet::from([key!("a", "r1", "m"), key!("b", "r2", "m")]);
    let head = BTreeSet::from([key!("a", "r1", "m"), key!("c", "r3", "new finding")]);
    assert_eq!(
        lost(&Findings {
            base: base.clone(),
            head
        }),
        vec![key!("b", "r2", "m")],
        "the finding the head lacks"
    );
    let more: BTreeSet<FindingKey> = base
        .iter()
        .cloned()
        .chain([key!("c", "r3", "new finding")])
        .collect();
    assert_eq!(
        lost(&Findings { base, head: more }),
        Vec::<FindingKey>::new(),
        "gaining findings is fine"
    );
}

#[test]
fn reason_points_at_rule_source_line() {
    assert_eq!(
        check_reason_points_at_rule_source_line(),
        Ok(()),
        "reason_points_at_rule_source_line"
    );
}

fn check_reason_points_at_rule_source_line() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    scratch.write(
        &RelPath::of("src/rules.rs"),
        &Text::of("fn a() {}\nconst X: &str = \"no-unwrap\";\n"),
    )?;
    let reasons = to_reasons(
        &[LostFinding {
            input: Name::of("public"),
            key: key!("src/a.rs", "no-unwrap", "unwrap"),
        }],
        scratch.path(),
    )?;
    let first = reasons.first();
    ensure_eq!(
        first.map(|reason| (reason.code, reason.path.clone(), reason.line)),
        Some((
            Code::LostFinding,
            Some(RelPath::of("src/rules.rs")),
            Count::new(2)
        )),
        "points at the literal"
    );
    ensure!(
        first.is_some_and(|reason| reason
            .evidence
            .as_str()
            .contains("rule no-unwrap lost on public:src/a.rs")),
        "evidence names rule, input and path"
    );
    let missing = rule_source(scratch.path(), &Text::of("unknown-rule"))?;
    ensure_eq!(missing.path, RelPath::of("src/rules.rs"), "fallback path");
    ensure_eq!(missing.line, Count::new(1), "fallback line");
    Ok(())
}

/// A ustar archive with one member, as raw bytes.
fn tar_with(name: &Text, kind: Glyph) -> Bytes {
    let field = |text: &str, width: usize| Bytes::of(text.as_bytes()).padded_to(Count::new(width));
    let regular = kind.get() == '0';
    let size = if regular {
        "00000000001"
    } else {
        "00000000000"
    };
    let header = |checksum: &str| {
        Bytes::join(&[
            field(name.as_str(), 100),
            field("0000644", 8),
            field("0000000", 8),
            field("0000000", 8),
            field(size, 12),
            field("00000000000", 12),
            field(checksum, 8),
            field(&kind.get().to_string(), 1),
            field(if regular { "" } else { "/etc/passwd" }, 100),
            field("ustar", 6),
            field("00", 2),
        ])
        .padded_to(Count::new(512))
    };
    let sum = header("        ").byte_sum();
    let sealed = header(&format!("{:06o}\0 ", sum.get()));
    let data = if regular {
        field("x", 512)
    } else {
        Bytes::empty()
    };
    Bytes::join(&[sealed, data, Bytes::empty().padded_to(Count::new(1024))])
}

fn unpack_of(scratch: &Scratch, archive: &Bytes) -> Result<Unpack, Fail> {
    let file = scratch.path().join("in.tar");
    io::write_bytes(&file, archive)?;
    Ok(Unpack {
        archive: file,
        dest: scratch.path().join("out"),
    })
}

#[test]
fn safe_extract_rejects_traversal_and_links() {
    assert_eq!(
        check_safe_extract_rejects_traversal_and_links(),
        Ok(()),
        "safe_extract_rejects_traversal_and_links"
    );
}

fn check_safe_extract_rejects_traversal_and_links() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    for (name, kind) in [("../evil", '0'), ("/abs", '0'), ("ln", '2')] {
        let job = unpack_of(&scratch, &tar_with(&Text::of(name), Glyph::new(kind)))?;
        ensure!(
            safe_extract(&job).is_err(),
            "{name} ({kind}) must be refused"
        );
    }
    let job = unpack_of(&scratch, &tar_with(&Text::of("ok/a.rs"), Glyph::new('0')))?;
    safe_extract(&job)?;
    ensure_eq!(
        io::is_file(&job.dest.join("ok").join("a.rs")),
        Answer::Yes,
        "a plain member is extracted"
    );
    Ok(())
}

#[test]
fn lock_line_grammar() {
    let good = format!(
        "https://static.crates.io/crates/x/x-1.crate {} x-1",
        "a".repeat(64)
    );
    assert!(
        parse_lock_line(&Text::from_string(good)).is_some(),
        "a good line parses"
    );
    let plain = format!("http://x {} n", "a".repeat(64));
    assert!(
        parse_lock_line(&Text::from_string(plain)).is_none(),
        "http is refused"
    );
    let short = format!("https://x {} n", "a".repeat(63));
    assert!(
        parse_lock_line(&Text::from_string(short)).is_none(),
        "a short digest is refused"
    );
}

#[test]
fn real_corpus_lock_parses() {
    let lock = Text::of(include_str!("../../../policy/corpus.lock"));
    let lines = parse_lock(&lock);
    assert!(
        lines.is_ok_and(|found| !found.is_empty()),
        "the pinned corpus lock parses"
    );
}

#[test]
fn compare_with_fake_binaries() {
    assert_eq!(
        check_compare_with_fake_binaries(),
        Ok(()),
        "compare_with_fake_binaries"
    );
}

fn write_script(scratch: &Scratch, name: &Name, body: &Text) -> Result<PathBuf, Fail> {
    let path = scratch.path().join(name.as_str());
    io::write_text(
        &path,
        &Text::from_string(format!("#!/bin/sh\n{}", body.as_str())),
    )?;
    process::make_executable(&path)?;
    Ok(path)
}

fn check_compare_with_fake_binaries() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    let base = script!(
        scratch,
        "base",
        "echo \"src/a.rs:1:1: r1: m1\"; echo \"src/b.rs:1:1: r2: m2\"; exit 1\n"
    )?;
    let head = script!(
        scratch,
        "head",
        "echo \"src/a.rs:5:1: r1: m1\"; echo \"src/z.rs:1:1: r9: extra\"; exit 1\n"
    )?;
    let root = scratch.path().join("in");
    io::create_dirs(&root)?;
    let done = compare(
        &Tools {
            base,
            head,
            prefix: Vec::new(),
        },
        &[input_of(&Name::of("sample"), Origin::Public, &root)?],
    )?;
    ensure_eq!(
        done.lost,
        vec![LostFinding {
            input: Name::of("sample"),
            key: key!("src/b.rs", "r2", "m2"),
        }],
        "only the lost finding"
    );
    ensure_eq!(done.errors.len(), 0, "no errors");
    ensure_eq!(
        done.counts.get(&Name::of("base_findings_sample")),
        Count::new(2),
        "base findings counted"
    );
    Ok(())
}

/// An input with fresh scratch build directories.
fn input_of(name: &Name, origin: Origin, root: &Path) -> Result<Input, Fail> {
    Ok(Input {
        name: name.clone(),
        origin,
        root: root.to_path_buf(),
        targets: TargetDirs {
            base: process::make_scratch_dir()?,
            head: process::make_scratch_dir()?,
        },
    })
}

#[test]
fn config_less_input_is_evaluated() {
    assert_eq!(
        check_config_less_input_is_evaluated(),
        Ok(()),
        "config_less_input_is_evaluated"
    );
}

fn check_config_less_input_is_evaluated() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    scratch.write(&RelPath::of("a.rs"), &Text::of("fn a() {}\n"))?;
    let workspace = make_workspace(scratch.path(), Origin::Public)?;
    ensure!(
        workspace != scratch.path(),
        "an input without a config gets a workspace of its own"
    );
    let base = write_script(&scratch, &Name::of("base"), &Text::of(NEEDS_WORKSPACE))?;
    let head = write_script(&scratch, &Name::of("head"), &Text::of(NEEDS_WORKSPACE))?;
    let input = input_of(&Name::of("plain"), Origin::Public, &workspace)?;
    let clearing = vec![
        Text::of("env"),
        Text::of("-i"),
        Text::of("PATH=/usr/bin:/bin"),
    ];
    let done = compare(
        &Tools {
            base,
            head,
            prefix: clearing,
        },
        slice::from_ref(&input),
    )?;
    ensure_eq!(
        done.errors,
        Vec::<Text>::new(),
        "both binaries evaluated it"
    );
    ensure_eq!(done.lost.len(), 0, "nothing was lost");
    ensure_eq!(
        done.counts.get(&Name::of("base_findings_plain")),
        Count::new(1),
        "the base finding is counted"
    );
    ensure_eq!(
        io::is_file(&input.targets.base.join("probe")),
        Answer::Yes,
        "the base binary built in its scratch directory"
    );
    ensure_eq!(
        io::is_file(&input.targets.head.join("probe")),
        Answer::Yes,
        "the head binary built in its scratch directory, through a sandbox that clears the environment"
    );
    let config = io::read_text(&workspace.join(".unrepresentable/config"))?;
    ensure!(
        config.as_str().contains("default-range = \"HEAD\""),
        "the config names a range, or the base binary exits 2"
    );
    io::remove_tree(&workspace)?;
    Ok(())
}

#[test]
fn crates_and_trees_become_members() {
    assert_eq!(
        check_crates_and_trees_become_members(),
        Ok(()),
        "crates_and_trees_become_members"
    );
}

fn check_crates_and_trees_become_members() -> Result<(), Fail> {
    let tree = Scratch::repository()?;
    tree.write(&RelPath::of("deep/b.rs"), &Text::of("fn b() {}\n"))?;
    tree.write(&RelPath::of("a.rs"), &Text::of("fn a() {}\n"))?;
    let workspace = make_workspace(tree.path(), Origin::Public)?;
    let lib = io::read_text(&workspace.join("src/lib.rs"))?;
    ensure!(
        lib.as_str().contains("#[path = \"tree/a.rs\"]")
            && lib.as_str().contains("#[path = \"tree/deep/b.rs\"]"),
        "every file of a bare tree is a module: {}",
        lib.as_str()
    );
    ensure!(
        io::read_text(&workspace.join("Cargo.lock"))?
            .as_str()
            .contains("name = \"corpus\""),
        "the lock names the member"
    );
    let count = process::git(&workspace, &Args::of(&["rev-list", "--count", "HEAD"]))?;
    let commits = count.lossy_text();
    ensure_eq!(commits.as_str().trim(), "1", "one commit");
    io::remove_tree(&workspace)?;
    let crates = Scratch::repository()?;
    crates.write(
        &RelPath::of("syn/syn-1/Cargo.toml"),
        &Text::of("[package]\nname = \"syn\"\n[dependencies]\nquote = \"1\"\n"),
    )?;
    crates.write(
        &RelPath::of("syn/syn-1/src/lib.rs"),
        &Text::of("pub fn s() {}\n"),
    )?;
    crates.write(
        &RelPath::of("other/Cargo.toml"),
        &Text::of("[package]\nname = \"other\"\n"),
    )?;
    crates.write(
        &RelPath::of("other/src/main.rs"),
        &Text::of("fn main() {}\n"),
    )?;
    let made = make_workspace(crates.path(), Origin::Public)?;
    let manifest = io::read_text(&made.join("crates/corpus-001/Cargo.toml"))?;
    ensure!(
        !manifest.as_str().contains("quote"),
        "the generated manifest has no dependencies"
    );
    ensure_eq!(
        io::is_file(&made.join("crates/corpus-000/src/main.rs")),
        Answer::Yes,
        "each crate is a member"
    );
    let config = io::read_text(&made.join(".unrepresentable/config"))?;
    ensure!(
        config.as_str().contains("[member.corpus-000]")
            && config.as_str().contains("[member.corpus-001]"),
        "every member has a role"
    );
    io::remove_tree(&made)?;
    Ok(())
}

#[test]
fn failed_evaluation_is_an_error_not_a_pass() {
    assert_eq!(
        check_failed_evaluation_is_an_error_not_a_pass(),
        Ok(()),
        "failed_evaluation_is_an_error_not_a_pass"
    );
}

fn check_failed_evaluation_is_an_error_not_a_pass() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    let clean = script!(scratch, "clean", "echo \"src/a.rs:1:1: r1: m1\"\nexit 1\n")?;
    let broken = script!(
        scratch,
        "broken",
        "echo \"error: no change range\" >&2\nexit 2\n"
    )?;
    let root = scratch.path().join("in");
    io::create_dirs(&root)?;
    let input = input_of(&Name::of("plain"), Origin::Public, &root)?;
    let run = |base: &PathBuf, head: &PathBuf| {
        compare(
            &Tools {
                base: base.clone(),
                head: head.clone(),
                prefix: Vec::new(),
            },
            slice::from_ref(&input),
        )
    };
    let head_failed = run(&clean, &broken)?;
    ensure_eq!(
        head_failed.errors,
        vec![Text::of(
            "head binary exit 2 on input plain: error: no change range"
        )],
        "a head that fails is an error even when the base found nothing, and says why"
    );
    let base_failed = run(&broken, &clean)?;
    ensure_eq!(
        base_failed.errors,
        vec![Text::of(
            "base binary exit 2 on input plain: error: no change range"
        )],
        "a base that fails is an error, and says why"
    );
    ensure_eq!(
        run(&clean, &clean)?.errors,
        Vec::<Text>::new(),
        "two runs that complete are not errors"
    );
    Ok(())
}

#[test]
fn input_with_its_own_config_is_used_unchanged() {
    assert_eq!(
        check_input_with_its_own_config_is_used_unchanged(),
        Ok(()),
        "input_with_its_own_config_is_used_unchanged"
    );
}

fn check_input_with_its_own_config_is_used_unchanged() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    scratch.write(
        &RelPath::of(".unrepresentable/config"),
        &Text::of("baseline = \"forbidden\"\n"),
    )?;
    ensure_eq!(
        make_workspace(scratch.path(), Origin::Hidden)?,
        scratch.path().to_path_buf(),
        "a trusted input with its own config is the workspace"
    );
    ensure_eq!(
        io::is_file(&scratch.path().join("Cargo.lock")),
        Answer::No,
        "nothing is added to it"
    );
    ensure_eq!(
        io::is_file(&scratch.path().join(".unrepresentable/ledger.md")),
        Answer::No,
        "no ledger is written into it"
    );
    Ok(())
}

fn run_pair(input: &Input, pair: (&PathBuf, &PathBuf)) -> Result<super::Comparison, Fail> {
    Ok(compare(
        &Tools {
            base: pair.0.clone(),
            head: pair.1.clone(),
            prefix: vec![
                Text::of("env"),
                Text::of("-i"),
                Text::of("PATH=/usr/bin:/bin"),
            ],
        },
        slice::from_ref(input),
    )?)
}

#[test]
fn an_empty_or_unrecognised_base_run_is_an_error() {
    assert_eq!(
        check_an_empty_or_unrecognised_base_run_is_an_error(),
        Ok(()),
        "an_empty_or_unrecognised_base_run_is_an_error"
    );
}

fn check_an_empty_or_unrecognised_base_run_is_an_error() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    let root = scratch.path().join("in");
    io::create_dirs(&root)?;
    let good = printing!(scratch, "good", ["range: HEAD", "src/a.rs:1:1: r1: m1"])?;
    let empty = script!(scratch, "empty", "exit 0\n")?;
    let drifted = printing!(
        scratch,
        "drifted",
        ["src/a.rs line 1 [r1] m1", "src/b.rs line 2 [r2] m2"]
    )?;
    let mixed = printing!(
        scratch,
        "mixed",
        ["src/a.rs:1:1: r1: m1", "something else entirely"]
    )?;
    let plain = input_of(&Name::of("plain"), Origin::Public, &root)?;
    ensure_eq!(
        run_pair(&plain, (&good, &good))?.errors,
        Vec::<Text>::new(),
        "one finding and the tool's own range line is a real run"
    );
    let on_empty = run_pair(&plain, (&empty, &good))?.errors;
    ensure!(
        on_empty.len() == 1
            && on_empty.iter().all(|line| line
                .as_str()
                .contains("reported 0 findings on input plain, fewer than the 1 expected")),
        "an empty corpus is an error, not a pass: {on_empty:?}"
    );
    ensure!(
        !run_pair(&plain, (&drifted, &good))?.errors.is_empty(),
        "a changed output format is an error"
    );
    let on_mixed = run_pair(&plain, (&mixed, &good))?.errors;
    ensure!(
        on_mixed
            .iter()
            .all(|line| line.as_str().contains("1 lines that are not findings")),
        "a line that is not a finding is an error: {on_mixed:?}"
    );
    ensure_eq!(on_mixed.len(), 1, "one error for the mixed output");
    check_minimums(&root, (&good, &empty))
}

fn check_minimums(root: &Path, scripts: (&PathBuf, &PathBuf)) -> Result<(), Fail> {
    let (good, empty) = scripts;
    let fixtures = input_of(&Name::of("fixtures"), Origin::Head, root)?;
    let hidden = input_of(&Name::of("hidden"), Origin::Hidden, root)?;
    let on_fixtures = run_pair(&fixtures, (good, good))?.errors;
    ensure!(
        on_fixtures
            .iter()
            .any(|line| line.as_str().contains("fewer than the 100 expected")),
        "the fixtures need a hundred findings: {on_fixtures:?}"
    );
    let on_hidden = run_pair(&hidden, (empty, good))?.errors;
    ensure!(
        on_hidden.len() == 1
            && on_hidden
                .iter()
                .all(|line| !line.as_str().contains(|found: char| found.is_ascii_digit())),
        "the error for the hidden corpus names no number: {on_hidden:?}"
    );
    Ok(())
}

#[test]
fn findings_lost_on_the_hidden_corpus_are_only_counted() {
    assert_eq!(
        check_findings_lost_on_the_hidden_corpus_are_only_counted(),
        Ok(()),
        "findings_lost_on_the_hidden_corpus_are_only_counted"
    );
}

fn check_findings_lost_on_the_hidden_corpus_are_only_counted() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    let root = scratch.path().join("in");
    io::create_dirs(&root)?;
    let base = printing!(
        scratch,
        "base",
        [
            "secret/private.rs:1:1: secret-rule: secret message",
            "secret/other.rs:2:2: other-rule: other message",
        ]
    )?;
    let head = printing!(
        scratch,
        "head",
        ["secret/private.rs:1:1: secret-rule: secret message"]
    )?;
    let hidden = input_of(&Name::of("hidden"), Origin::Hidden, &root)?;
    let done = run_pair(&hidden, (&base, &head))?;
    ensure_eq!(done.errors, Vec::<Text>::new(), "no error");
    ensure_eq!(
        done.lost,
        Vec::<LostFinding>::new(),
        "no finding of the hidden corpus is kept"
    );
    ensure_eq!(done.hidden_lost, Count::new(1), "the number is kept");
    ensure_eq!(
        done.counts.contains(&Name::of("base_findings_hidden")),
        Answer::No,
        "the size of the hidden corpus is not reported"
    );
    let reasons = hidden_reasons(done.hidden_lost);
    ensure_eq!(reasons.len(), 1, "one reason");
    let shown = format!("{reasons:?}");
    for secret in ["private", "other", "secret", "message", "rule"] {
        ensure!(
            !shown.contains(secret),
            "the reason does not leak {secret}: {shown}"
        );
    }
    ensure_eq!(
        reasons
            .first()
            .map(|reason| (reason.code, reason.path.clone())),
        Some((Code::LostFindingHidden, None)),
        "a general reason with the hidden code"
    );
    ensure_eq!(
        hidden_reasons(Count::new(0)).len(),
        0,
        "nothing lost, no reason"
    );
    Ok(())
}

#[test]
fn an_input_from_the_pull_request_never_keeps_its_own_configuration() {
    assert_eq!(
        check_an_input_from_the_pull_request_never_keeps_its_own_configuration(),
        Ok(()),
        "an_input_from_the_pull_request_never_keeps_its_own_configuration"
    );
}

fn check_an_input_from_the_pull_request_never_keeps_its_own_configuration() -> Result<(), Fail> {
    let tree = Scratch::repository()?;
    for (name, text) in HOSTILE_TREE {
        tree.write(&RelPath::of(name), &Text::of(text))?;
    }
    let made = make_workspace(tree.path(), Origin::Head)?;
    ensure!(
        made != tree.path(),
        "an input from the pull request is never the workspace, whatever it contains"
    );
    for gone in [
        ".cargo",
        "rust-toolchain.toml",
        "build.rs",
        "src/tree/.cargo",
        "src/tree/build.rs",
        "src/tree/rust-toolchain.toml",
        "src/tree/Cargo.toml",
        "src/tree/Cargo.lock",
        "src/tree/.unrepresentable",
        "src/tree/deep/.cargo",
        "src/tree/deep/build.rs",
        "src/tree/deep/rust-toolchain",
    ] {
        ensure_eq!(
            io::exists_no_follow(&made.join(gone)),
            Answer::No,
            "{gone} is not in the workspace"
        );
    }
    ensure_eq!(
        io::exists_no_follow(&made.join("src/tree/deep/b.rs")),
        Answer::Yes,
        "the sources stay"
    );
    for generated in ["Cargo.toml", ".unrepresentable/config", "Cargo.lock"] {
        let text = io::read_text(&made.join(generated))?;
        ensure!(
            !text.as_str().contains("evil"),
            "{generated} is generated, not the input's"
        );
    }
    io::remove_tree(&made)?;
    Ok(())
}

#[test]
fn a_crate_from_the_pull_request_gets_a_generated_manifest() {
    assert_eq!(
        check_crates_are_regenerated(),
        Ok(()),
        "a_crate_from_the_pull_request_gets_a_generated_manifest"
    );
}

fn check_crates_are_regenerated() -> Result<(), Fail> {
    let crates = Scratch::repository()?;
    for (name, text) in [
        (
            "c/Cargo.toml",
            "[package]\nname = \"c\"\nbuild = \"b.rs\"\n",
        ),
        ("c/build.rs", "fn main() {}\n"),
        ("c/.cargo/config.toml", "evil\n"),
        ("c/rust-toolchain.toml", "evil\n"),
        ("c/src/lib.rs", "pub fn c() {}\n"),
        ("c/src/build.rs", "fn main() {}\n"),
    ] {
        crates.write(&RelPath::of(name), &Text::of(text))?;
    }
    let made = make_workspace(crates.path(), Origin::Head)?;
    for gone in [
        "crates/corpus-000/build.rs",
        "crates/corpus-000/.cargo",
        "crates/corpus-000/rust-toolchain.toml",
        "crates/corpus-000/src/build.rs",
    ] {
        ensure_eq!(
            io::exists_no_follow(&made.join(gone)),
            Answer::No,
            "{gone} is not in the workspace"
        );
    }
    let manifest = io::read_text(&made.join("crates/corpus-000/Cargo.toml"))?;
    ensure!(
        manifest.as_str().contains("build = false")
            && !manifest
                .as_str()
                .replace("build = false", "")
                .contains("build"),
        "the manifest of a member is generated and has no build script, only `build = false`"
    );
    ensure_eq!(
        io::exists_no_follow(&made.join("crates/corpus-000/src/lib.rs")),
        Answer::Yes,
        "the crate source stays"
    );
    io::remove_tree(&made)?;
    Ok(())
}

#[test]
fn archive_listings_are_paired_safely() {
    let names = Text::of("ok/\nok/a.rs\n");
    let details = Text::of(
        "drwxr-xr-x u/g 0 2020-01-01 00:00 ok/\n-rw-r--r-- u/g 5 2020-01-01 00:00 ok/a.rs\n",
    );
    assert!(
        check_listing(&Listing { details, names }).is_ok(),
        "a good listing"
    );
    for (bad_names, bad_details, why) in [
        (
            "ok/\nok/a\u{7}.rs\n",
            "drwxr-xr-x u/g 0 2020-01-01 00:00 ok/\n-rw-r--r-- u/g 5 2020-01-01 00:00 ok/a\u{7}.rs\n",
            "a control character in a name",
        ),
        (
            "ok/\nok/a.rs\n",
            "drwxr-xr-x u/g 0 2020-01-01 00:00 ok/\n-rw-r--r-- u/g 5 2020-01-01 00:00 other.rs\n",
            "a detail line that does not end with its name",
        ),
        (
            "ok/a\nb\n",
            "-rw-r--r-- u/g 5 2020-01-01 00:00 ok/a\n",
            "a name that spans two lines",
        ),
        (
            "../x\n",
            "-rw-r--r-- u/g 5 2020-01-01 00:00 ../x\n",
            "a name that leaves the tree",
        ),
        (
            "ln\n",
            "lrwxrwxrwx u/g 0 2020-01-01 00:00 ln -> /etc/passwd\n",
            "a link",
        ),
    ] {
        assert!(
            check_listing(&Listing {
                details: Text::of(bad_details),
                names: Text::of(bad_names)
            })
            .is_err(),
            "{why} is refused"
        );
    }
}

#[test]
fn the_two_binaries_run_as_different_users_in_directories_of_their_own() {
    assert_eq!(
        check_the_two_binaries_run_as_different_users_in_directories_of_their_own(),
        Ok(()),
        "the_two_binaries_run_as_different_users_in_directories_of_their_own"
    );
}

fn check_the_two_binaries_run_as_different_users_in_directories_of_their_own() -> Result<(), Fail> {
    ensure!(
        process::BASE_UID != process::HEAD_UID,
        "the base and the head binary run as different users"
    );
    let scratch = Scratch::repository()?;
    let log = scratch.path().join("sandbox.log");
    // A stand-in for sandbox.sh: records the user and directory the caller
    // named in its own environment, then runs the program.
    let sandbox = write_script(
        &scratch,
        &Name::of("sandbox"),
        &Text::from_string(format!(
            "echo \"$SANDBOX_UID $SANDBOX_SCRATCH\" >> '{}'\nexec \"$@\"\n",
            log.display()
        )),
    )?;
    let base = printing!(scratch, "base", ["src/a.rs:1:1: r1: m1"])?;
    let head = printing!(scratch, "head", ["src/a.rs:1:1: r1: m1"])?;
    let root = scratch.path().join("in");
    io::create_dirs(&root)?;
    let first = input_of(&Name::of("first"), Origin::Public, &root)?;
    let second = input_of(&Name::of("second"), Origin::Public, &root)?;
    let done = compare(
        &Tools {
            base,
            head,
            prefix: vec![Text::from_string(sandbox.to_string_lossy().into_owned())],
        },
        &[first.clone(), second.clone()],
    )?;
    ensure_eq!(done.errors, Vec::<Text>::new(), "both runs completed");
    let expected: Vec<String> = [
        (process::BASE_UID, &first.targets.base),
        (process::HEAD_UID, &first.targets.head),
        (process::BASE_UID, &second.targets.base),
        (process::HEAD_UID, &second.targets.head),
    ]
    .iter()
    .map(|(uid, dir)| format!("{} {}", uid.get(), dir.display()))
    .collect();
    let logged: Vec<String> = io::read_text(&log)?
        .as_str()
        .lines()
        .map(str::to_owned)
        .collect();
    ensure_eq!(
        logged,
        expected,
        "each run is told its own user and its own fresh directory, base first"
    );
    ensure_user_dirs(&[
        &first.targets.base,
        &first.targets.head,
        &second.targets.base,
        &second.targets.head,
    ])?;
    Ok(())
}

/// No two of `dirs` are the same, and each has the parts a run needs.
fn ensure_user_dirs(dirs: &[&PathBuf]) -> Result<(), Fail> {
    let distinct: BTreeSet<&PathBuf> = dirs.iter().copied().collect();
    ensure_eq!(distinct.len(), dirs.len(), "no two runs share a directory");
    for dir in dirs {
        for part in ["cargo-home", "home", "tmp"] {
            ensure_eq!(
                io::is_dir(&dir.join(part)),
                Answer::Yes,
                "the scratch directory has {part}"
            );
        }
    }
    Ok(())
}

/// The error of `base` failing on `root` as an input of the given origin.
fn base_error(scratch: &Scratch, body: &Text, origin: Origin) -> Result<Vec<Text>, Fail> {
    let failing = write_script(scratch, &Name::of("failing"), body)?;
    let root = scratch.path().join("in");
    io::create_dirs(&root)?;
    let input = input_of(&Name::of("plain"), origin, &root)?;
    Ok(run_pair(&input, (&failing, &failing))?.errors)
}

#[test]
fn a_failed_run_says_why_except_on_the_hidden_corpus() {
    assert_eq!(
        check_a_failed_run_says_why_except_on_the_hidden_corpus(),
        Ok(()),
        "a_failed_run_says_why_except_on_the_hidden_corpus"
    );
}

/// The failure of the first pull-request run was only "base binary exit 2 on
/// input public": the tool's own message was thrown away.
fn check_a_failed_run_says_why_except_on_the_hidden_corpus() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    let body = Text::of(
        "echo 'error: failed to run custom build command' >&2\n\
         echo 'could not execute process' >&2\nexit 2\n",
    );
    for origin in [Origin::Public, Origin::Head] {
        ensure_eq!(
            base_error(&scratch, &body, origin)?,
            vec![Text::of(
                "base binary exit 2 on input plain: error: failed to run custom build command | could not execute process"
            )],
            "{origin:?}: the first lines of standard error are in the error"
        );
    }
    ensure_eq!(
        base_error(&scratch, &body, Origin::Hidden)?,
        vec![Text::of("base binary exit 2 on input plain")],
        "the hidden corpus gets the exit status and nothing the tool said"
    );
    ensure_eq!(
        base_error(&scratch, &Text::of("exit 2\n"), Origin::Public)?,
        vec![Text::of("base binary exit 2 on input plain")],
        "a tool that said nothing adds nothing"
    );
    Ok(())
}

#[test]
fn what_a_failed_run_said_is_safe_to_print() {
    assert_eq!(
        check_what_a_failed_run_said_is_safe_to_print(),
        Ok(()),
        "what_a_failed_run_said_is_safe_to_print"
    );
}

fn check_what_a_failed_run_said_is_safe_to_print() -> Result<(), Fail> {
    let scratch = Scratch::repository()?;
    let body = Text::of(
        "printf '::error::boom\\n\\033[31mred\\033[0m\\r\\n' >&2\n\
         printf '%0400d\\n' 0 >&2\n\
         i=0\n\
         while [ \"$i\" -lt 30 ]; do echo \"line $i\" >&2; i=$((i+1)); done\n\
         exit 2\n",
    );
    let errors = base_error(&scratch, &body, Origin::Public)?;
    ensure_eq!(errors.len(), 1, "one error");
    let shown = errors.iter().map(Text::as_str).collect::<String>();
    ensure!(
        shown.starts_with("base binary exit 2 on input plain: "),
        "the error starts with fixed text, so no workflow command can start in it: {shown}"
    );
    ensure!(
        shown
            .chars()
            .all(|found| found.is_ascii_graphic() || found == ' '),
        "only printable characters, no escape, newline or carriage return: {shown:?}"
    );
    ensure!(shown.contains("red"), "the text itself is kept: {shown}");
    let Some(excerpt) = shown.strip_prefix("base binary exit 2 on input plain: ") else {
        return Err(Fail::new(&Message::of("the excerpt has no fixed start")));
    };
    let segments: Vec<&str> = excerpt.split(" | ").collect();
    ensure_eq!(
        segments.len(),
        12,
        "exactly twelve lines are kept (eleven separators) of the thirty-three the script wrote: {shown}"
    );
    ensure_eq!(
        segments.last().copied(),
        Some("line 8"),
        "the lines kept are the first twelve of the non-empty ones"
    );
    ensure!(
        segments
            .iter()
            .all(|segment| segment.chars().count() <= 200),
        "no line is longer than 200 characters: {shown}"
    );
    ensure_eq!(
        segments.iter().map(|segment| segment.chars().count()).max(),
        Some(200),
        "the line of 400 characters was cut at exactly 200"
    );
    Ok(())
}

#[test]
fn a_generated_manifest_never_runs_a_build_script() {
    assert_eq!(
        check_a_generated_manifest_never_runs_a_build_script(),
        Ok(()),
        "a_generated_manifest_never_runs_a_build_script"
    );
}

/// The failure behind the first pull-request run: `proc-macro2` of the public
/// corpus has a `build.rs`, cargo found it although the generated manifest did
/// not name it, and `cargo clippy` (run by the tool for the lint tables) built
/// and ran it, which failed in the sandbox and made the base binary exit 2.
fn check_a_generated_manifest_never_runs_a_build_script() -> Result<(), Fail> {
    for origin in [Origin::Public, Origin::Head] {
        let crates = Scratch::repository()?;
        crates.write(
            &RelPath::of("pm/Cargo.toml"),
            &Text::of("[package]\nname = \"pm\"\nversion = \"1.0.0\"\n"),
        )?;
        crates.write(&RelPath::of("pm/src/lib.rs"), &Text::of("pub fn p() {}\n"))?;
        crates.write(
            &RelPath::of("pm/build.rs"),
            &Text::of("fn main() { std::process::exit(1); }\n"),
        )?;
        let made = make_workspace(crates.path(), origin)?;
        let manifest = io::read_text(&made.join("crates/corpus-000/Cargo.toml"))?;
        ensure!(
            manifest.as_str().contains("build = false"),
            "{origin:?}: the manifest turns the build script off: {}",
            manifest.as_str()
        );
        let target = crates.path().join("target");
        let program = Name::of("cargo");
        let args = Args::of(&["check", "--offline", "--locked", "--quiet"]);
        let captured = process::run(
            &process::Invocation::new(&program, &args)
                .in_dir(&made)
                .with_env(&process::EnvPair {
                    name: Name::of("CARGO_TARGET_DIR"),
                    value: Text::from_string(target.to_string_lossy().into_owned()),
                }),
        )?;
        ensure_eq!(
            captured.status,
            process::Status::Code(Count::new(0)),
            "{origin:?}: cargo checks the member although build.rs would fail: {}",
            captured.stderr.lossy_text().as_str()
        );
        io::remove_tree(&made)?;
    }
    let tree = Scratch::repository()?;
    tree.write(&RelPath::of("a.rs"), &Text::of("pub fn a() {}\n"))?;
    let made = make_workspace(tree.path(), Origin::Public)?;
    ensure!(
        io::read_text(&made.join("Cargo.toml"))?
            .as_str()
            .contains("build = false"),
        "the member of a bare tree has no build script either"
    );
    io::remove_tree(&made)?;
    Ok(())
}

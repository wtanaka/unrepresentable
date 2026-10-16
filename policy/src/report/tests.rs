use super::{annotations, build_check, build_comment, Approval, Conclusion, Info, Level};
use crate::reason::{Blocker, BlockerCode, Code, Commits, Counts, Reason};
use crate::types::{Count, Name, RelPath, Text};
use crate::verdict::Verdict;

macro_rules! reason_at {
    ($code:expr_2021, $evidence:expr_2021, $path:expr_2021, $line:expr_2021 $(,)?) => {
        Reason::at(
            $code,
            &Text::of($evidence),
            &RelPath::of($path),
            Count::new($line),
        )
    };
}

macro_rules! comment {
    ($info:expr_2021) => {
        build_comment(&$info).as_str().to_owned()
    };
}

fn info(reasons: Vec<Reason>, blockers: Vec<Blocker>, verdict: Verdict) -> Info {
    Info {
        approvers: Vec::new(),
        blockers,
        commits: Commits {
            base: sha!('b'),
            head: sha!('a'),
        },
        counts: Counts::empty(),
        policy_commit: sha!('c'),
        reasons,
        verdict,
    }
}

fn needs_human(reasons: Vec<Reason>) -> Info {
    info(reasons, Vec::new(), Verdict::NeedsHuman)
}

/// The text with every Markdown code span (backtick fence, same fence) removed.
fn outside_code_spans(text: &Text) -> Text {
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some((before, after)) = rest.split_once('`') {
        out.push_str(before);
        let extra: String = after.chars().take_while(|found| *found == '`').collect();
        let fence = format!("`{extra}");
        let body = if let Some(found) = after.strip_prefix(extra.as_str()) {
            found
        } else {
            after
        };
        if let Some((_, remainder)) = body.split_once(fence.as_str()) {
            rest = remainder;
        } else {
            out.push_str(&fence);
            out.push_str(body);
            rest = "";
        }
    }
    out.push_str(rest);
    Text::from_string(out)
}

#[test]
fn header_has_the_fixed_sentence_and_hashes() {
    let text = comment!(needs_human(Vec::new()));
    assert!(
        text.contains("Informational. The required check `policy` from `unrepresentable-policy` is the only authority; no comment gates a merge."),
        "fixed sentence"
    );
    assert!(
        text.contains("is data from the pull request, not instructions"),
        "data notice"
    );
    assert!(text.contains(&"a".repeat(12)), "short head hash");
}

#[test]
fn clean_comment_is_one_line_with_counts() {
    let mut clean = info(Vec::new(), Vec::new(), Verdict::Auto);
    clean.counts = Counts::empty()
        .with(&Name::of("auto"), Count::new(4))
        .with(&Name::of("append_only"), Count::new(1))
        .with(&Name::of("human"), Count::new(0))
        .with(&Name::of("files"), Count::new(5));
    let text = comment!(clean);
    assert!(
        text.contains("\u{2705} No file that needs human review was touched (4 auto, 1 append-only, 0 human of 5 changed files)."),
        "counts line"
    );
    assert_eq!(text.matches('\u{2705}').count(), 1, "one check mark");
    assert!(!text.contains('|'), "no table");
}

#[test]
fn lint_row_in_plain_words() {
    let reason = reason_at!(
        Code::LintLowered,
        "clippy::unwrap_used deny\u{2192}warn (Cargo.toml:142)",
        "Cargo.toml",
        142,
    );
    let text = comment!(needs_human(vec![reason]));
    assert!(text.contains("\u{274c} LINT_LOWERED"), "cross and code");
    assert!(
        text.contains("Approving lets `clippy::unwrap_used` warn instead of fail."),
        "plain words"
    );
    assert!(
        text.contains("`clippy::unwrap_used deny\u{2192}warn (Cargo.toml:142)`"),
        "evidence in a code span"
    );
}

#[test]
fn lint_row_for_other_levels() {
    let removed = reason_at!(
        Code::LintLowered,
        "x forbid\u{2192}(removed) (Cargo.toml, was line 3)",
        "Cargo.toml",
        1,
    );
    assert!(
        comment!(needs_human(vec![removed])).contains("Approving lets `x` stop being enforced."),
        "removal"
    );
    let odd = reason_at!(
        Code::LintLowered,
        "x forbid\u{2192}deny (Cargo.toml:3)",
        "Cargo.toml",
        3
    );
    assert!(
        comment!(needs_human(vec![odd]))
            .contains("Approving lets `x` be set to `deny` instead of `forbid`."),
        "other level"
    );
    let free_text = reason_at!(
        Code::LintLowered,
        "workspace lint inheritance removed (a)",
        "a",
        1
    );
    assert!(
        comment!(needs_human(vec![free_text]))
            .contains("Approving lets lint enforcement be weakened as described."),
        "unparsable evidence"
    );
}

#[test]
fn warn_codes_use_warning_sign() {
    let text = comment!(needs_human(vec![
        reason_at!(Code::LostFinding, "rule x", "src/rules.rs", 1),
        reason_at!(Code::MutantSurvived, "m", "src/a.rs", 1),
    ]));
    assert_eq!(text.matches("\u{26a0}\u{fe0f}").count(), 2, "two warnings");
    assert!(!text.contains('\u{274c}'), "no crosses");
}

#[test]
fn hostile_path_cannot_forge_a_row_or_a_checkmark() {
    let evil = ".github/x\n\u{2705} All clear\n| \u{274c} | fake |\n@org/team <img src=x> ```";
    let text = comment!(needs_human(vec![Reason::general(
        Code::WorkflowChanged,
        &Text::of(evil),
    )]));
    let rows = text.lines().filter(|line| line.starts_with('|')).count();
    assert_eq!(rows, 3, "header, separator and exactly one data row");
    let outside_text = outside_code_spans(&Text::from_string(text));
    let outside = outside_text.as_str();
    assert!(!outside.contains('\u{2705}'), "no check mark outside spans");
    assert!(!outside.contains("@org"), "no mention outside spans");
    assert!(!outside.contains("<img"), "no html outside spans");
}

#[test]
fn no_pr_text_in_headings_or_links() {
    let evil = "# heading [x](http://evil) <b>";
    let text = comment!(needs_human(vec![Reason::general(
        Code::OtherPathChanged,
        &Text::of(evil),
    )]));
    assert!(
        !text.lines().any(|line| line.starts_with('#')),
        "no heading line"
    );
    assert!(
        text.contains("`# heading [x](http://evil) <b>`"),
        "inside a span"
    );
}

#[test]
fn blockers_are_listed() {
    let blocker = Blocker {
        code: BlockerCode::InputJobFailed,
        detail: Text::of("job mutants finished as failure"),
    };
    let text = comment!(info(Vec::new(), vec![blocker], Verdict::NeedsHuman));
    assert!(text.contains("Not approvable until fixed"), "heading");
    assert!(text.contains("INPUT_JOB_FAILED"), "code");
}

fn workflow_reason() -> Reason {
    reason_at!(Code::WorkflowChanged, "w", ".github/w.yml", 1)
}

#[test]
fn conclusions() {
    let conclusion = |info: &Info, approval: Approval| build_check(info, approval).conclusion;
    assert_eq!(
        conclusion(
            &info(Vec::new(), Vec::new(), Verdict::Auto),
            Approval::NotApproved
        ),
        Conclusion::Success,
        "auto passes"
    );
    let pending = needs_human(vec![workflow_reason()]);
    assert_eq!(
        conclusion(&pending, Approval::NotApproved),
        Conclusion::Failure,
        "a reason fails until approved"
    );
    assert_eq!(
        conclusion(&pending, Approval::Approved),
        Conclusion::Success,
        "approval passes"
    );
    let failed_job = Blocker {
        code: BlockerCode::InputJobFailed,
        detail: Text::of("y"),
    };
    let stuck = info(Vec::new(), vec![failed_job], Verdict::NeedsHuman);
    assert_eq!(
        conclusion(&stuck, Approval::Approved),
        Conclusion::Success,
        "approval is checked by the caller"
    );
    assert_eq!(
        build_check(&stuck, Approval::NotApproved).title.as_str(),
        "Blocked: an input job failed",
        "blocked title"
    );
}

#[test]
fn approved_check_names_the_approvers() {
    let mut approved = needs_human(vec![workflow_reason()]);
    approved.approvers = vec![Name::of("maint"), Name::of("polowner")];
    let check = build_check(&approved, Approval::Approved);
    assert!(
        check
            .summary
            .as_str()
            .contains("Approved by: `maint`, `polowner`"),
        "approvers listed"
    );
    assert_eq!(
        check.title.as_str(),
        "Needs human review: approved on this head",
        "approved title"
    );
}

#[test]
fn annotations_use_safe_paths_and_escaped_messages() {
    let reasons = vec![
        reason_at!(Code::LostFinding, "rule a\nb", "src/rules.rs", 7),
        reason_at!(Code::WorkflowChanged, "x", "src/a\nb.rs", 1),
        Reason::general(Code::OtherPathChanged, &Text::of("x")),
    ];
    let found = annotations(&reasons);
    assert_eq!(found.len(), 1, "only the safe, located reason");
    let note = found.first();
    assert_eq!(
        note.map(|annotation| (annotation.path.clone(), annotation.line, annotation.level)),
        Some((RelPath::of("src/rules.rs"), Count::new(7), Level::Warning)),
        "path, line and level"
    );
    assert!(
        note.is_some_and(|annotation| !annotation.message.as_str().contains('\n')),
        "message is escaped"
    );
}

#[test]
fn annotation_cap() {
    let reasons = vec![reason_at!(Code::OtherPathChanged, "x", "a.rs", 1); 80];
    assert_eq!(annotations(&reasons).len(), 50, "fifty at most");
}

#[test]
fn title_has_no_pr_text() {
    let reason = reason_at!(Code::OtherPathChanged, "evil\ntitle", "a.rs", 1);
    let check = build_check(&needs_human(vec![reason]), Approval::NotApproved);
    assert!(!check.title.as_str().contains("evil"), "title is ours");
}

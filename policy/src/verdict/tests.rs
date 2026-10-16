use super::{
    compute, merge_message, parse_block, verdict_block, Approvable, BlockFacts, Decision,
    ParsedBlock, PullNumber, Verdict,
};
use crate::failure::Failure;
use crate::reason::{Blocker, BlockerCode, Code, Commits, Reason};
use crate::testkit::Fail;
use crate::types::{Message, Name, Sha, Text};

fn facts(codes: Vec<Code>, approvers: Vec<Name>) -> BlockFacts {
    BlockFacts {
        approvers,
        codes,
        commits: Commits {
            base: sha!('b'),
            head: sha!('a'),
        },
        policy_commit: sha!('c'),
    }
}

fn some_reason() -> Reason {
    Reason::general(Code::OtherPathChanged, &Text::of("x"))
}

#[test]
fn compute_decides_auto_only_without_reasons_and_blockers() {
    assert_eq!(
        compute(&[], &[]),
        Decision {
            approvable: Approvable::Yes,
            verdict: Verdict::Auto
        },
        "nothing to review is auto"
    );
    assert_eq!(
        compute(&[some_reason()], &[]),
        Decision {
            approvable: Approvable::Yes,
            verdict: Verdict::NeedsHuman
        },
        "a reason needs a human who can approve"
    );
    let blocker = Blocker {
        code: BlockerCode::InputJobFailed,
        detail: Text::of("y"),
    };
    assert_eq!(
        compute(&[], &[blocker]),
        Decision {
            approvable: Approvable::No,
            verdict: Verdict::NeedsHuman
        },
        "a blocker cannot be approved away"
    );
}

#[test]
fn merge_message_ends_with_verdict_block() {
    assert_eq!(
        check_merge_message_ends_with_verdict_block(),
        Ok(()),
        "merge_message_ends_with_verdict_block"
    );
}

fn check_merge_message_ends_with_verdict_block() -> Result<(), Fail> {
    let Some(number) = PullNumber::parse(&Text::of("12")) else {
        return Err(Fail::new(&Message::of("12 is a pull request number")));
    };
    let message = merge_message(number, &facts(Vec::new(), Vec::new()))?;
    ensure_eq!(
        message.subject.as_str(),
        "Squash merge of pull request #12",
        "neutral subject"
    );
    ensure!(
        message
            .body
            .as_str()
            .trim_end_matches('\n')
            .ends_with("approver: none"),
        "the block ends the message"
    );
    ensure_eq!(
        parse_block(&message.body),
        Some(ParsedBlock {
            approver: Text::of("none"),
            base: sha!('b'),
            head: sha!('a'),
            policy_commit: sha!('c'),
            reasons: Text::of(""),
        }),
        "the block parses back"
    );
    Ok(())
}

#[test]
fn human_merge_block_lists_approver_and_codes() {
    assert_eq!(
        check_human_merge_block_lists_approver_and_codes(),
        Ok(()),
        "human_merge_block_lists_approver_and_codes"
    );
}

fn check_human_merge_block_lists_approver_and_codes() -> Result<(), Fail> {
    let Some(number) = PullNumber::parse(&Text::of("12")) else {
        return Err(Fail::new(&Message::of("12 is a pull request number")));
    };
    let message = merge_message(
        number,
        &facts(
            vec![Code::WorkflowChanged, Code::LintLowered],
            vec![Name::of("maint")],
        ),
    )?;
    ensure!(
        message
            .body
            .as_str()
            .contains("reasons: LINT_LOWERED,WORKFLOW_CHANGED"),
        "codes are sorted"
    );
    ensure!(
        message.body.as_str().contains("approver: maint"),
        "approver is named"
    );
    Ok(())
}

#[test]
fn validation() {
    for bad in ["", "0", "12a", "1 2", "-1", "99999999999", "1; rm"] {
        assert!(
            PullNumber::parse(&Text::of(bad)).is_none(),
            "{bad:?} is not a pull request number"
        );
    }
    assert!(PullNumber::parse(&Text::of("12")).is_some(), "12 is");
    for bad in [
        "A".repeat(40),
        "a".repeat(39),
        format!("{}\n", "a".repeat(40)),
    ] {
        assert!(
            Sha::parse(&Text::from_string(bad.clone())).is_none(),
            "{bad:?}"
        );
    }
    assert_eq!(
        verdict_block(&facts(Vec::new(), vec![Name::of("bad login\n")])),
        Err::<Text, Failure>(Failure::refused(&Message::of("bad approver login"))),
        "a login with a newline cannot enter a commit message"
    );
}

#[test]
fn block_parsing_is_strict() {
    let good = "subject\n\nunrepresentable-policy-verdict: v1\nbase: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\nhead: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\npolicy-commit: cccccccccccccccccccccccccccccccccccccccc\nreasons: LINT_LOWERED\napprover: maint\n";
    assert!(
        parse_block(&Text::of(good)).is_some(),
        "the model block parses"
    );
    let junk_after = format!("{good}more\n");
    assert!(
        parse_block(&Text::from_string(junk_after)).is_none(),
        "text after the block is refused"
    );
    let lower = good.replace("LINT_LOWERED", "lint_lowered");
    assert!(
        parse_block(&Text::from_string(lower)).is_none(),
        "codes are upper case"
    );
}

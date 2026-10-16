use super::{Reviewed, VerdictData};
use crate::failure::Failure;
use crate::json;
use crate::reason::{Blocker, BlockerCode, Code, Commits, Counts, Job, Reason};
use crate::report::Approval;
use crate::types::{Count, Name, RelPath, Text};
use crate::verdict::{Approvable, PullNumber, Verdict};

fn sample() -> VerdictData {
    VerdictData {
        approvable: Approvable::Yes,
        attempt: Count::new(2),
        blockers: vec![Blocker {
            code: BlockerCode::SettingsAudit,
            detail: Text::of("stale"),
        }],
        commits: Commits {
            base: sha!('b'),
            head: sha!('a'),
        },
        counts: Counts::empty().with(&Name::of("files"), Count::new(2)),
        inputs: vec![
            (Job::Classify, Text::of("success")),
            (Job::Mutants, Text::of("failure")),
        ],
        policy_commit: sha!('c'),
        pull: PullNumber::from_count(Count::new(7)),
        reasons: vec![
            Reason::at(
                Code::WorkflowChanged,
                &Text::of("a\n\u{2705} b"),
                &RelPath::of(".github/x.yml"),
                Count::new(3),
            ),
            Reason::general(Code::UnsafePathChars, &Text::of("esc")),
        ],
        reviewed: None,
        run_id: Count::new(5),
        verdict: Verdict::NeedsHuman,
    }
}

fn roundtrip(data: &VerdictData) -> Result<VerdictData, Failure> {
    let parsed =
        json::parse(&data.to_json().compact()).map_err(|message| Failure::parse(&message))?;
    VerdictData::from_json(&parsed)
}

#[test]
fn verdict_data_roundtrips() {
    let data = sample();
    assert_eq!(roundtrip(&data), Ok(data), "data survives JSON");
}

#[test]
fn review_record_roundtrips() {
    let data = VerdictData {
        reviewed: Some(Reviewed {
            approval: Approval::Approved,
            approvers: vec![Name::of("maint")],
        }),
        ..sample()
    };
    assert_eq!(roundtrip(&data), Ok(data), "review record survives JSON");
}

#[test]
fn damaged_data_is_refused() {
    let text = Text::of("{\"v\":1,\"pr\":7}");
    let parsed = json::parse(&text).map(|value| VerdictData::from_json(&value));
    assert!(matches!(parsed, Ok(Err(_))), "missing fields are refused");
}

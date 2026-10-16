use super::{emit, parse, Parsed};
use crate::bytes::Bytes;
use crate::failure::Failure;
use crate::reason::{Code, Commits, Counts, Job, Reason};
use crate::testkit::Fail;
use crate::types::{Count, Message, Name, RelPath, Text};
use core::{iter, slice};

macro_rules! refusal {
    ($why:literal) => {
        Err(Failure::parse(&Message::of($why)))
    };
}

/// One more byte than the size limit allows.
const TOO_BIG: usize = 0x1_0001;

/// More reasons than the line limit allows.
const TOO_MANY: usize = 1005;

fn commits() -> Commits {
    Commits {
        base: sha!('b'),
        head: sha!('a'),
    }
}

fn files(count: Count) -> Counts {
    Counts::empty().with(&Name::of("files"), count)
}

fn roundtrip(job: Job, reasons: &[Reason], counts: &Counts) -> Result<Parsed, Failure> {
    let text = emit(job, &commits(), reasons, counts);
    parse(&Bytes::of_text(&text), job, &commits())
}

fn empty_artifact(job: Job) -> Text {
    emit(job, &commits(), &[], &Counts::empty())
}

fn parse_text(text: &Text) -> Result<Parsed, Failure> {
    parse(&Bytes::of_text(text), Job::Classify, &commits())
}

#[test]
fn roundtrip_preserves_hostile_text() {
    let evil = Text::of("src/a\n\u{2705} ok\u{202e}.rs");
    let reason = Reason::at(
        Code::WorkflowChanged,
        &evil,
        &RelPath::of("x"),
        Count::new(3),
    );
    assert_eq!(
        roundtrip(
            Job::Classify,
            slice::from_ref(&reason),
            &files(Count::new(3))
        ),
        Ok(Parsed {
            counts: files(Count::new(3)),
            reasons: vec![reason],
        }),
        "reasons and counts survive unchanged"
    );
}

#[test]
fn reason_without_a_path_roundtrips() {
    let reason = Reason::general(Code::UnsafePathChars, &Text::of("escaped"));
    assert_eq!(
        roundtrip(Job::Classify, slice::from_ref(&reason), &Counts::empty()),
        Ok(Parsed {
            counts: Counts::empty(),
            reasons: vec![reason],
        }),
        "no path stays no path"
    );
}

#[test]
fn wrong_head_or_job_rejected() {
    let other = Commits {
        base: sha!('b'),
        head: sha!('c'),
    };
    assert_eq!(
        parse(
            &Bytes::of_text(&empty_artifact(Job::Classify)),
            Job::Classify,
            &other
        ),
        refusal!("header does not match the evaluated run"),
        "another head is refused"
    );
    assert_eq!(
        parse(
            &Bytes::of_text(&empty_artifact(Job::Classify)),
            Job::Mutants,
            &commits()
        ),
        refusal!("header does not match the evaluated run"),
        "another job is refused"
    );
}

#[test]
fn job_may_not_emit_foreign_code() {
    let lost = Reason::general(Code::LostFinding, &Text::of("x"));
    assert_eq!(
        roundtrip(Job::Mutants, &[lost], &Counts::empty()),
        refusal!("job mutants may not emit LOST_FINDING"),
        "mutants cannot report a lost finding"
    );
    let survived = Reason::general(Code::MutantSurvived, &Text::of("x"));
    assert_eq!(
        roundtrip(Job::HeadChecks, &[survived], &Counts::empty()),
        refusal!("job head-checks may not emit MUTANT_SURVIVED"),
        "head-checks may not report anything"
    );
}

#[test]
fn size_limit() {
    let big = Bytes::from_vec(vec![b'x'; TOO_BIG]);
    assert_eq!(
        parse(&big, Job::Classify, &commits()),
        refusal!("artifact larger than 65536 bytes"),
        "oversized artifact"
    );
}

#[test]
fn garbage_is_rejected() {
    assert_eq!(check_garbage_is_rejected(), Ok(()), "garbage_is_rejected");
}

fn check_garbage_is_rejected() -> Result<(), Fail> {
    let good = empty_artifact(Job::Classify);
    let text = good.as_str().to_owned();
    let mut non_ascii = text.clone().into_bytes();
    non_ascii.push(0xff);
    let bad = [
        Bytes::of(text.replace("\nend\n", "\nextra\nend\n").as_bytes()),
        Bytes::of(text.trim_end_matches('\n').as_bytes()),
        Bytes::of(text.replace("end\n", "").as_bytes()),
        Bytes::of(format!("{text}end\n").as_bytes()),
        Bytes::of(b"R X\n"),
        Bytes::of(text.replace("v1", "v2").as_bytes()),
        Bytes::of(text.replace("end", "end\r").as_bytes()),
        Bytes::from_vec(non_ascii),
    ];
    for sample in bad {
        ensure!(
            parse(&sample, Job::Classify, &commits()).is_err(),
            "garbage must be refused: {sample:?}"
        );
    }
    Ok(())
}

#[test]
fn duplicate_count_keys_rejected() {
    assert_eq!(
        check_duplicate_count_keys_rejected(),
        Ok(()),
        "duplicate_count_keys_rejected"
    );
}

fn check_duplicate_count_keys_rejected() -> Result<(), Fail> {
    let good = emit(Job::Classify, &commits(), &[], &files(Count::new(1)));
    let doubled = good.as_str().replace("end\n", "N files 2\nend\n");
    ensure_eq!(
        parse_text(&Text::from_string(doubled)),
        refusal!("unparsable line"),
        "a key may appear once"
    );
    Ok(())
}

#[test]
fn bad_base64_rejected() {
    assert_eq!(check_bad_base64_rejected(), Ok(()), "bad_base64_rejected");
}

fn check_bad_base64_rejected() -> Result<(), Fail> {
    let good = empty_artifact(Job::Classify);
    let broken = good
        .as_str()
        .replace("end\n", "R WORKFLOW_CHANGED !!!! - 1\nend\n");
    ensure_eq!(
        parse_text(&Text::from_string(broken)),
        refusal!("unparsable line"),
        "the evidence must be base64"
    );
    Ok(())
}

#[test]
fn too_many_lines() {
    let many: Vec<Reason> = iter::repeat_n(
        Reason::general(Code::OtherPathChanged, &Text::of("p")),
        TOO_MANY,
    )
    .collect();
    assert_eq!(
        roundtrip(Job::Classify, &many, &Counts::empty()),
        refusal!("line count out of range"),
        "the line cap holds"
    );
}

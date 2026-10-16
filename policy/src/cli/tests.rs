use super::args::Arguments;
use super::{run, FIRST, SECOND};
use crate::exit::ExitStatus;
use crate::io::Vars;
use crate::types::{Name, Text};

macro_rules! words {
    ($($word:literal),* $(,)?) => {
        [$(Text::of($word)),*]
    };
}

#[test]
fn flags_and_positional_words_are_split() {
    let parsed = Arguments::parse(&words![
        "a",
        "--out",
        "x",
        "--input=n=d",
        "--input",
        "m=e",
        "b"
    ]);
    let Ok(args) = parsed else {
        assert!(parsed.is_ok(), "the arguments parse");
        return;
    };
    assert_eq!(
        args.optional(&Name::of("out")),
        Some(Text::of("x")),
        "a flag value"
    );
    assert_eq!(
        args.all(&Name::of("input")),
        vec![Text::of("n=d"), Text::of("m=e")],
        "repeated flags keep their order"
    );
    assert_eq!(args.word(FIRST), Ok(Text::of("a")), "first word");
    assert_eq!(args.word(SECOND), Ok(Text::of("b")), "second word");
    assert!(args.word(super::THIRD).is_err(), "no third word");
}

#[test]
fn a_flag_without_a_value_is_refused() {
    assert!(
        Arguments::parse(&words!["--out"]).is_err(),
        "--out needs a value"
    );
}

#[test]
fn unknown_commands_and_no_command_are_usage_errors() {
    assert!(run(&[], &Vars::empty()).is_err(), "no command");
    assert!(
        run(&words!["bogus"], &Vars::empty()).is_err(),
        "unknown command"
    );
}

#[test]
fn lock_check_reports_through_the_exit_status() {
    let status = run(&words!["lock-check", "../Cargo.lock"], &Vars::empty());
    assert_eq!(
        status,
        Ok(ExitStatus::Clean),
        "this repository's lock is registry-only"
    );
}

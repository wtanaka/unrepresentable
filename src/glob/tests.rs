use super::{Brackets, Glob};
use crate::types::{Answer, Holds as _, RelPath, Text};

macro_rules! glob_matches {
    ($pattern:literal, $path:literal) => {
        matched(&Text::of($pattern), &RelPath::of($path), Brackets::Literal)
    };
}

macro_rules! class_matches {
    ($pattern:literal, $path:literal) => {
        matched(&Text::of($pattern), &RelPath::of($path), Brackets::Class)
    };
}

/// Whether `pattern` (config reading: `[` literal) matches `path`; a pattern
/// that does not parse matches nothing, and the parse tests pin those.
fn matched(pattern: &Text, path: &RelPath, brackets: Brackets) -> Answer {
    match Glob::parse(pattern, brackets) {
        Ok(glob) => glob.matches(path),
        Err(_) => Answer::No,
    }
}

#[test]
fn star_stays_within_one_segment() {
    assert_eq!(
        glob_matches!("tests/*.rs", "tests/a.rs"),
        Answer::Yes,
        "a file directly in the directory"
    );
    assert_eq!(
        glob_matches!("tests/*.rs", "tests/x/a.rs"),
        Answer::No,
        "a star does not cross a slash"
    );
    assert_eq!(
        glob_matches!("tests/*.rs", "tests/a.rs.txt"),
        Answer::No,
        "the suffix must end the name"
    );
}

#[test]
fn double_star_segment_matches_zero_or_more_segments() {
    assert_eq!(
        glob_matches!("src/**", "src/a/b.rs"),
        Answer::Yes,
        "deep file"
    );
    assert_eq!(
        glob_matches!("src/**", "src/a.rs"),
        Answer::Yes,
        "one segment"
    );
    assert_eq!(
        glob_matches!("src/**", "src"),
        Answer::Yes,
        "a final double star also matches zero segments"
    );
    assert_eq!(
        glob_matches!("src/**", "srcx/a.rs"),
        Answer::No,
        "other directory"
    );
    assert_eq!(
        glob_matches!("**/Cargo.toml", "Cargo.toml"),
        Answer::Yes,
        "zero leading directories"
    );
    assert_eq!(
        glob_matches!("**/Cargo.toml", "a/b/Cargo.toml"),
        Answer::Yes,
        "two leading directories"
    );
    assert_eq!(
        glob_matches!("a/**/b", "a/b"),
        Answer::Yes,
        "zero middle directories"
    );
    assert_eq!(
        glob_matches!("a/**/b", "a/x/y/b"),
        Answer::Yes,
        "two middle directories"
    );
    assert_eq!(
        glob_matches!("**", "any/path/at/all"),
        Answer::Yes,
        "everything"
    );
}

#[test]
fn trailing_slash_matches_a_directory_prefix() {
    assert_eq!(
        glob_matches!("tests/canaries/", "tests/canaries/a/b.rs"),
        Answer::Yes,
        "below the directory"
    );
    assert_eq!(
        glob_matches!("tests/canaries/", "tests/canaries"),
        Answer::Yes,
        "the directory itself, as the earlier matcher did"
    );
    assert_eq!(
        glob_matches!("tests/canaries/", "tests/canariesx/a.rs"),
        Answer::No,
        "a different directory"
    );
}

#[test]
fn question_mark_is_one_character_within_a_segment() {
    assert_eq!(glob_matches!("a?c", "abc"), Answer::Yes, "one character");
    assert_eq!(glob_matches!("a?c", "a/c"), Answer::No, "not a slash");
    assert_eq!(
        glob_matches!("a?c", "ac"),
        Answer::No,
        "not zero characters"
    );
}

#[test]
fn double_star_inside_a_segment_is_a_single_star() {
    assert_eq!(
        glob_matches!("a**b", "axb"),
        Answer::Yes,
        "inside one segment"
    );
    assert_eq!(
        glob_matches!("a**b", "a/b"),
        Answer::No,
        "never crosses a slash"
    );
}

#[test]
fn bracket_is_a_literal_character() {
    assert_eq!(
        glob_matches!("a[b]", "a[b]"),
        Answer::Yes,
        "the literal text"
    );
    assert_eq!(
        glob_matches!("a[b]", "ab"),
        Answer::No,
        "not a character class"
    );
    assert_eq!(
        glob_matches!("a[b", "a[b"),
        Answer::Yes,
        "an unclosed bracket"
    );
}

#[test]
fn matching_is_case_sensitive_and_byte_for_byte() {
    assert_eq!(
        glob_matches!("src/A.rs", "src/a.rs"),
        Answer::No,
        "case differs"
    );
    assert_eq!(
        glob_matches!(".github/**", ".github/workflows/x.yml"),
        Answer::Yes,
        "a leading dot needs no special pattern"
    );
}

#[test]
fn a_leading_slash_is_an_error_not_a_literal() {
    assert_eq!(
        Glob::parse(&Text::of("/a"), Brackets::Literal)
            .is_err()
            .answer(),
        Answer::Yes,
        "a repo-relative path never starts with a slash, so the glob could not match"
    );
    assert_eq!(
        Glob::parse(&Text::of("/a"), Brackets::Class)
            .is_err()
            .answer(),
        Answer::Yes,
        "member patterns too"
    );
}

#[test]
fn an_empty_glob_is_an_error() {
    assert_eq!(
        Glob::parse(&Text::of(""), Brackets::Literal)
            .is_err()
            .answer(),
        Answer::Yes,
        "empty"
    );
}

#[test]
fn a_trailing_slash_means_the_directory_and_what_is_below() {
    assert_eq!(
        glob_matches!("a/", "a"),
        Answer::Yes,
        "the directory itself (or a file of that name)"
    );
    assert_eq!(
        glob_matches!("a/", "a/b/c"),
        Answer::Yes,
        "below the directory"
    );
    assert_eq!(
        glob_matches!("a/", "ab/c"),
        Answer::No,
        "a different directory"
    );
}

#[test]
fn a_character_class_is_read_only_for_member_patterns() {
    assert_eq!(
        class_matches!("crates/[ab]", "crates/a"),
        Answer::Yes,
        "a class member"
    );
    assert_eq!(
        class_matches!("crates/[ab]", "crates/c"),
        Answer::No,
        "outside the class"
    );
    assert_eq!(
        class_matches!("crates/[!ab]*", "crates/cx"),
        Answer::Yes,
        "a negated class"
    );
    assert_eq!(
        Glob::parse(&Text::of("crates/[ab"), Brackets::Class)
            .is_err()
            .answer(),
        Answer::Yes,
        "an unclosed class is an error"
    );
    assert_eq!(
        Glob::parse(&Text::of("crates/[ab"), Brackets::Literal)
            .is_err()
            .answer(),
        Answer::No,
        "the config reading keeps it literal"
    );
}

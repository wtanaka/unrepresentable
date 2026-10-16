use super::{owners_for, parse};
use crate::types::{Name, RelPath, Text};

macro_rules! owners {
    ($rules:expr_2021, $path:expr_2021) => {
        match parse(&Text::of($rules)) {
            Ok(rules) => owners_for(&rules, &RelPath::of($path)),
            Err(failure) => vec![Name::from_string(failure.text().as_str().to_owned())],
        }
    };
}

macro_rules! names {
    ($($item:expr_2021),* $(,)?) => {
        vec![$(Name::of($item)),*]
    };
}

#[test]
fn star_matches_everything() {
    assert_eq!(owners!("* @a\n", "src/x/y.rs"), names!["@a"], "nested path");
    assert_eq!(owners!("* @a\n", "top"), names!["@a"], "top-level file");
}

#[test]
fn leading_slash_anchors_a_directory() {
    assert_eq!(
        owners!("/policy/ @p\n", "policy/a.py"),
        names!["@p"],
        "below the directory"
    );
    assert_eq!(
        owners!("/policy/ @p\n", "x/policy/a.py"),
        Vec::<Name>::new(),
        "not a nested directory of that name"
    );
    assert_eq!(
        owners!("/policy/ @p\n", "policy"),
        Vec::<Name>::new(),
        "a trailing slash needs something below"
    );
}

#[test]
fn inner_slash_anchors_too() {
    assert_eq!(
        owners!("docs/a/ @d\n", "docs/a/b.md"),
        names!["@d"],
        "root docs"
    );
    assert_eq!(
        owners!("docs/a/ @d\n", "x/docs/a/b.md"),
        Vec::<Name>::new(),
        "nested docs are not matched"
    );
    assert_eq!(
        owners!("docs/ @d\n", "x/docs/b.md"),
        names!["@d"],
        "a lone trailing slash does not anchor"
    );
}

#[test]
fn bare_names_match_at_any_depth() {
    assert_eq!(owners!("*.rs @r\n", "a.rs"), names!["@r"], "top level");
    assert_eq!(
        owners!("*.rs @r\n", "src/deep/a.rs"),
        names!["@r"],
        "nested"
    );
    assert_eq!(
        owners!("*.rs @r\n", "src/a.rs.txt"),
        Vec::<Name>::new(),
        "extension must end the name"
    );
    assert_eq!(
        owners!("foo @f\n", "x/foo/bar"),
        names!["@f"],
        "a directory"
    );
}

#[test]
fn double_star_forms() {
    assert_eq!(owners!("/src/** @s\n", "src/a/b.rs"), names!["@s"], "below");
    assert_eq!(
        owners!("**/foo.txt @f\n", "foo.txt"),
        names!["@f"],
        "zero dirs"
    );
    assert_eq!(
        owners!("**/foo.txt @f\n", "a/b/foo.txt"),
        names!["@f"],
        "two dirs"
    );
}

#[test]
fn question_mark_never_crosses_a_slash() {
    assert_eq!(owners!("/a?c @q\n", "abc"), names!["@q"], "one character");
    assert_eq!(
        owners!("/a?c @q\n", "a/c"),
        Vec::<Name>::new(),
        "a slash is not a character"
    );
}

#[test]
fn comments_blank_lines_and_non_owners_are_ignored() {
    assert_eq!(
        owners!("# c\n\n* @a plain # trailing\n", "x"),
        names!["@a"],
        "only @words are owners"
    );
}

#[test]
fn bracket_is_literal_and_double_star_inside_a_name_is_a_star() {
    assert_eq!(
        owners!("/a[b] @l\n", "a[b]"),
        names!["@l"],
        "a bracket is not a character class"
    );
    assert_eq!(
        owners!("/a[b] @l\n", "ab"),
        Vec::<Name>::new(),
        "so it does not match the class members"
    );
    assert_eq!(
        owners!("/a**b @s\n", "axb"),
        names!["@s"],
        "a double star inside a name is one star"
    );
    assert_eq!(
        owners!("/a**b @s\n", "a/b"),
        Vec::<Name>::new(),
        "which never crosses a slash"
    );
}

#[test]
fn last_matching_rule_wins_and_a_later_narrower_rule_overrides() {
    let rules = "* @all\n/policy/ @p\n*.md @docs\n";
    assert_eq!(owners!(rules, "src/a.rs"), names!["@all"], "the catch-all");
    assert_eq!(owners!(rules, "policy/a.py"), names!["@p"], "the directory");
    assert_eq!(
        owners!(rules, "policy/README.md"),
        names!["@docs"],
        "the last matching rule wins"
    );
}

#[test]
fn a_directory_pattern_matches_below_it_and_a_name_matches_both() {
    assert_eq!(
        owners!("/docs @d\n", "docs/a/b.md"),
        names!["@d"],
        "a name without a slash at the end covers what is below"
    );
    assert_eq!(
        owners!("/docs @d\n", "docs"),
        names!["@d"],
        "and the name itself"
    );
    assert_eq!(
        owners!("/docs/** @d\n", "docs"),
        Vec::<Name>::new(),
        "a final double star needs something below"
    );
}

#[test]
fn double_star_between_directories_matches_zero_or_more() {
    assert_eq!(
        owners!("a/**/b @m\n", "a/b"),
        names!["@m"],
        "zero directories between"
    );
    assert_eq!(
        owners!("a/**/b @m\n", "a/x/y/b"),
        names!["@m"],
        "two directories between"
    );
}

#[test]
fn a_final_wildcard_matches_one_level_as_github_documents() {
    assert_eq!(
        owners!("docs/* @d\n", "docs/getting-started.md"),
        names!["@d"],
        "a file directly in docs"
    );
    assert_eq!(
        owners!("docs/* @d\n", "docs/build-app/troubleshooting.md"),
        Vec::<Name>::new(),
        "not a file in a nested directory"
    );
    assert_eq!(
        owners!("/docs/* @d\n", "docs/x/a.md"),
        Vec::<Name>::new(),
        "an anchored one-level pattern is not recursive either"
    );
    assert_eq!(
        owners!("*.js @j\n", "src/a.js"),
        names!["@j"],
        "an extension pattern matches at any depth"
    );
    assert_eq!(
        owners!("*.js @j\n", "src/a.js/b.txt"),
        Vec::<Name>::new(),
        "but not below a directory named like a file"
    );
}

#[test]
fn githubs_directory_examples() {
    assert_eq!(
        owners!("/build/logs/ @l\n", "build/logs/a.log"),
        names!["@l"],
        "a root directory"
    );
    assert_eq!(
        owners!("/build/logs/ @l\n", "x/build/logs/a.log"),
        Vec::<Name>::new(),
        "not a nested copy of it"
    );
    assert_eq!(
        owners!("apps/ @o\n", "x/apps/a"),
        names!["@o"],
        "a directory at any depth"
    );
    assert_eq!(
        owners!("apps/ @o\n/apps/github @g\n", "apps/github/x"),
        names!["@g"],
        "the later, narrower rule wins"
    );
}

use super::{with_head, Head};
use crate::types::Text;

fn work() -> Head {
    Head::Named(Text::of("work"))
}

#[test]
fn the_current_head_leaves_every_range_alone() {
    assert_eq!(
        with_head(&Text::of("main...HEAD"), &Head::Current),
        Text::of("main...HEAD")
    );
    assert_eq!(Head::Current.spec(), Text::of("HEAD"));
}

#[test]
fn a_named_head_replaces_the_right_side_of_a_range() {
    assert_eq!(
        with_head(&Text::of("main...HEAD"), &work()),
        Text::of("main...work")
    );
    assert_eq!(
        with_head(&Text::of("main..HEAD"), &work()),
        Text::of("main..work")
    );
    assert_eq!(
        with_head(&Text::of("HEAD~1...HEAD"), &work()),
        Text::of("HEAD~1...work")
    );
}

#[test]
fn a_bare_head_becomes_the_name() {
    assert_eq!(with_head(&Text::of("HEAD"), &work()), Text::of("work"));
}

#[test]
fn other_ranges_are_unchanged() {
    for range in [
        "main...topic",
        "HEAD...main",
        "main",
        "main..",
        "HEADX",
        "a...b..HEAD",
    ] {
        assert_eq!(
            with_head(&Text::of(range), &work()),
            Text::of(range),
            "{range}"
        );
    }
}

#[test]
fn the_spec_of_a_named_head_is_its_name() {
    assert_eq!(work().spec(), Text::of("work"));
}

use super::{has_unsafe_chars, render, render_text};
use crate::types::{Answer, Text};

macro_rules! rendered {
    ($text:expr_2021) => {
        render(&Text::of($text)).as_str().to_owned()
    };
}

#[test]
fn newline_and_fake_check_mark_is_one_inert_span() {
    let out = rendered!("a\n\u{2705} All clear");
    assert_eq!(out, "`a\\u{000A}\u{2705} All clear`", "escaped newline");
    assert!(!out.contains('\n'), "no raw newline survives");
}

#[test]
fn mention_is_inside_code_span() {
    assert_eq!(rendered!("@org/team"), "`@org/team`", "mention in a span");
}

#[test]
fn html_is_inside_code_span() {
    assert_eq!(rendered!("<img src=x>"), "`<img src=x>`", "html in a span");
}

#[test]
fn bidi_override_is_escaped() {
    let out = rendered!("\u{202e}sr.esab");
    assert_eq!(out, "`\\u{202E}sr.esab`", "escaped override");
    assert!(!out.contains('\u{202e}'), "no raw override survives");
}

#[test]
fn three_backticks_get_a_longer_fence() {
    let out = rendered!("```");
    assert!(out.starts_with("````"), "fence opens with four backticks");
    assert!(out.ends_with("````"), "fence closes with four backticks");
    assert_eq!(out, "```` ``` ````", "padded span");
}

#[test]
fn fence_longer_than_any_inner_run() {
    let nine = "`".repeat(9);
    for text in ["`", "``x``", "a`b```c", nine.as_str()] {
        let out = rendered!(text);
        let fence = out.chars().take_while(|found| *found == '`').count();
        let longest = text
            .split(|found| found != '`')
            .map(str::len)
            .max()
            .into_iter()
            .next();
        assert!(
            Some(fence) > longest,
            "fence is longer than every run in {text}"
        );
    }
}

#[test]
fn five_hundred_char_path_is_truncated_with_original_length() {
    let out = rendered!(&"x".repeat(500));
    assert!(out.contains(&"x".repeat(200)), "keeps the first 200");
    assert!(!out.contains(&"x".repeat(201)), "drops the rest");
    assert!(
        out.ends_with("(truncated, original length 500)"),
        "says how long it was"
    );
}

#[test]
fn every_unsafe_class_is_escaped() {
    let samples = [
        '\u{0}',
        '\u{1f}',
        '\u{7f}',
        '\u{80}',
        '\u{9f}',
        '\u{200b}',
        '\u{200f}',
        '\u{202a}',
        '\u{202e}',
        '\u{2066}',
        '\u{2069}',
        '\u{feff}',
        '|',
        '\u{fffd}',
        '\u{e0001}',
        '\u{34f}',
        '\u{115f}',
        '\u{1160}',
        '\u{180b}',
        '\u{180e}',
        '\u{2065}',
        '\u{206a}',
        '\u{206f}',
        '\u{3164}',
        '\u{fe00}',
        '\u{fe0f}',
        '\u{ffa0}',
        '\u{e0100}',
        '\u{e01ef}',
    ];
    for sample in samples {
        let out = rendered!(&format!("a{sample}b"));
        assert!(!out.contains(sample), "raw {sample:?} must not survive");
        let want = format!("\\u{{{:04X}}}", u32::from(sample));
        assert!(out.contains(&want), "escape {want} is present");
    }
}

#[test]
fn has_unsafe_chars_ignores_pipe_but_flags_controls() {
    assert_eq!(
        has_unsafe_chars(&Text::of("src/a|b.rs")),
        Answer::No,
        "pipe is fine in a path"
    );
    assert_eq!(
        has_unsafe_chars(&Text::of("src/a\nb.rs")),
        Answer::Yes,
        "newline is not"
    );
    assert_eq!(
        has_unsafe_chars(&Text::of("src/\u{202e}b.rs")),
        Answer::Yes,
        "override is not"
    );
    assert_eq!(
        has_unsafe_chars(&Text::of("src/caf\u{e9}.rs")),
        Answer::No,
        "accents are fine"
    );
}

#[test]
fn empty_string_renders_a_span() {
    assert_eq!(rendered!(""), "`  `", "empty span");
}

#[test]
fn render_text_has_no_fence() {
    assert_eq!(
        render_text(&Text::of("a\nb")).as_str(),
        "a\\u{000A}b",
        "plain text"
    );
}

#[test]
fn invisible_fillers_and_selectors_are_unsafe_in_paths() {
    for sample in [
        '\u{34f}',
        '\u{115f}',
        '\u{1160}',
        '\u{180b}',
        '\u{180c}',
        '\u{180d}',
        '\u{180e}',
        '\u{2065}',
        '\u{206a}',
        '\u{206f}',
        '\u{3164}',
        '\u{fe00}',
        '\u{fe0f}',
        '\u{ffa0}',
        '\u{e0100}',
        '\u{e01ef}',
    ] {
        assert_eq!(
            has_unsafe_chars(&Text::from_string(format!("src/a{sample}b.rs"))),
            Answer::Yes,
            "{sample:?} is unsafe"
        );
    }
    for neighbour in [
        '\u{34e}',
        '\u{1161}',
        '\u{180a}',
        '\u{2070}',
        '\u{3165}',
        '\u{fe10}',
        '\u{ff9f}',
        '\u{e01f0}',
    ] {
        assert_eq!(
            has_unsafe_chars(&Text::from_string(format!("src/a{neighbour}b.rs"))),
            Answer::No,
            "{neighbour:?} next to a range stays safe"
        );
    }
}

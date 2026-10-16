use super::{parse, Flag, Json};
use crate::types::{Count, Message, Name, Text};

macro_rules! parsed {
    ($text:expr_2021) => {
        parse(&Text::of($text))
    };
}

/// The value of `result`, or `Json::Null` when it did not parse, which fails
/// the assertions that follow.
fn value_of(result: Result<Json, Message>) -> Json {
    match result {
        Ok(found) => found,
        Err(_) => Json::Null,
    }
}

/// The member names of an object, or none for any other value.
fn keys_of(value: &Json) -> Vec<Name> {
    match value {
        Json::Object(members) => members.iter().map(|(name, _)| name.clone()).collect(),
        Json::Array(_) | Json::Bool(_) | Json::Null | Json::Number(_) | Json::Str(_) => Vec::new(),
    }
}

#[test]
fn values_are_read_and_object_members_come_out_sorted() {
    let value = value_of(parsed!(r#"{"b": [1, true, null, "x"], "a": {"c": false}}"#));
    assert_eq!(
        keys_of(&value),
        [Name::of("a"), Name::of("b")],
        "sorted by key, not in source order"
    );
    assert_eq!(
        value
            .dig(&[Name::of("a"), Name::of("c")])
            .and_then(Json::as_flag),
        Some(Flag::Off),
        "a nested boolean"
    );
    assert_eq!(
        value.get(&Name::of("b")).and_then(Json::as_array),
        Some(
            [
                Json::Number(Text::of("1")),
                Json::Bool(true),
                Json::Null,
                Json::Str(Text::of("x")),
            ]
            .as_slice()
        ),
        "the array members"
    );
}

#[test]
fn escapes_and_surrogate_pairs_are_decoded() {
    let slash = '\x5c';
    let text = format!(
        "\"a{slash}n{slash}t{slash}\"{slash}{slash}{slash}/{slash}u00e9{slash}ud83d{slash}ude00\""
    );
    let value = value_of(parsed!(text.as_str()));
    assert_eq!(
        value.as_text(),
        Some(&Text::of("a\n\t\"\\/\u{e9}\u{1f600}")),
        "all escapes decode"
    );
}

#[test]
fn counts_are_non_negative_integers() {
    assert_eq!(
        value_of(parsed!("42")).as_count(),
        Some(Count::new(42)),
        "an integer"
    );
    assert_eq!(
        value_of(parsed!("-1")).as_count(),
        None,
        "a negative number"
    );
}

#[test]
fn malformed_input_is_refused_with_a_position() {
    for bad in ["", "{", "[1,]", "{\"a\" 1}", "nul", "\"open", "1 2", "{} x"] {
        assert!(parsed!(bad).is_err(), "{bad:?} must be refused");
    }
    let message = match parsed!("[1,\n 2,,]") {
        Err(found) => found,
        Ok(_) => Message::of(""),
    };
    assert!(
        message.as_str().contains("line 2"),
        "the message names the line: {message:?}"
    );
}

#[test]
fn absurd_nesting_is_refused_instead_of_overflowing_the_stack() {
    let deep = format!("{}{}", "[".repeat(10_000), "]".repeat(10_000));
    assert!(
        parsed!(deep.as_str()).is_err(),
        "ten thousand levels are refused"
    );
}

#[test]
fn compact_output_has_sorted_keys_and_parses_back() {
    let value = Json::object_of(vec![
        (Name::of("b"), Json::count_of(Count::new(2))),
        (Name::of("a"), Json::text_of(&Text::of("q\"\n"))),
    ]);
    let text = value.compact();
    assert_eq!(
        text.as_str(),
        "{\"a\":\"q\\\"\\n\",\"b\":2}",
        "keys sorted, string escaped"
    );
    assert_eq!(
        parse(&text).map(|again| again.compact()),
        Ok(text),
        "the output parses back and compacts to the same text"
    );
}

#[test]
fn numbers_are_normalised_by_the_library_and_integers_keep_their_digits() {
    assert_eq!(
        parsed!("[1.0e2,-0,18446744073709551615,9007199254740993,0.10]")
            .map(|value| value.compact()),
        Ok(Text::of(
            "[100.0,-0.0,18446744073709551615,9007199254740993,0.1]"
        )),
        "a float is written back as an f64; an integer is exact"
    );
}

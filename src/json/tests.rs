use super::{parse, Flag, Json};
use crate::types::{Count, Message, Name, Source, Text};

macro_rules! parsed {
    ($text:expr_2021) => {
        parse(&Source::from_string($text.to_owned()))
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

/// The members of an object, or none for any other value.
fn members_of(value: &Json) -> Vec<(Name, Json)> {
    match value {
        Json::Object(members) => members.clone(),
        Json::Array(_) | Json::Bool(_) | Json::Null | Json::Number(_) | Json::Str(_) => Vec::new(),
    }
}

#[test]
fn objects_arrays_and_scalars_are_read() {
    let value = value_of(parsed!(r#"{"a": [1, true, null, "x"], "b": {"c": false}}"#));
    assert_eq!(
        value.get(&Name::of("a")).and_then(Json::as_array),
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
    assert_eq!(
        value
            .get(&Name::of("b"))
            .and_then(|inner| inner.get(&Name::of("c")))
            .and_then(Json::as_flag),
        Some(Flag::Off),
        "a nested boolean"
    );
}

#[test]
fn object_members_come_out_sorted_by_key() {
    let value = value_of(parsed!(r#"{"b": 1, "a": 2, "c": 3}"#));
    let keys: Vec<String> = members_of(&value)
        .iter()
        .map(|(name, _)| name.as_str().to_owned())
        .collect();
    assert_eq!(
        keys,
        ["a", "b", "c"],
        "keys are sorted, not in source order"
    );
}

#[test]
fn escapes_and_surrogate_pairs_are_decoded() {
    let slash = '\x5c';
    let text = format!(
        "\"a{slash}n{slash}t{slash}\"{slash}{slash}{slash}/{slash}u00e9{slash}ud83d{slash}ude00\""
    );
    let value = value_of(parsed!(text));
    assert_eq!(
        value.as_text(),
        Some(&Text::of("a\n\t\"\\/\u{e9}\u{1f600}")),
        "all escapes decode"
    );
}

#[test]
fn counts_read_only_non_negative_integers() {
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
    assert_eq!(value_of(parsed!("1.5")).as_count(), None, "a fraction");
}

#[test]
fn malformed_input_is_an_error_with_a_position() {
    for bad in ["", "{", "[1,]", "{\"a\" 1}", "nul", "\"open", "1 2", "{} x"] {
        assert!(parsed!(bad).is_err(), "{bad:?} must be refused");
    }
    let message = match parsed!("[1,\n 2,,]") {
        Err(found) => found.as_str().to_owned(),
        Ok(_) => String::new(),
    };
    assert!(
        message.contains("line 2"),
        "the message names the line: {message:?}"
    );
}

#[test]
fn absurd_nesting_is_refused_instead_of_overflowing_the_stack() {
    let deep = format!("{}{}", "[".repeat(10_000), "]".repeat(10_000));
    assert!(parsed!(deep).is_err(), "ten thousand levels are refused");
}

#[test]
fn numbers_are_normalised_by_the_library_and_integers_keep_their_digits() {
    let value = value_of(parsed!(
        "[1.0e2,-0,18446744073709551615,9007199254740993,0.10]"
    ));
    assert_eq!(
        value.as_array(),
        Some(
            [
                Json::Number(Text::of("100.0")),
                Json::Number(Text::of("-0.0")),
                Json::Number(Text::of("18446744073709551615")),
                Json::Number(Text::of("9007199254740993")),
                Json::Number(Text::of("0.1")),
            ]
            .as_slice()
        ),
        "a float is written back as an f64; an integer is exact"
    );
}

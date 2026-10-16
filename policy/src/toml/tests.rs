use super::{parse, Table, Value};
use crate::failure::Failure;
use crate::types::{Answer, Message, Name, Text};

macro_rules! parsed {
    ($text:expr_2021) => {
        parse(&Text::of($text))
    };
}

macro_rules! texts {
    ($($item:expr_2021),* $(,)?) => {
        vec![$(Text::of($item)),*]
    };
}

/// The table of `result`; one that did not parse is empty, which fails the
/// assertions that follow.
fn table_of(result: Result<Table, Failure>) -> Table {
    let Ok(found) = result else {
        return Table::empty();
    };
    found
}

/// The failure message of `result`, empty when it parsed.
fn message_of(result: Result<Table, Failure>) -> Message {
    match result {
        Err(failure) => failure.text(),
        Ok(_) => Message::of(""),
    }
}

/// `key@line` for each entry, in order.
fn keys_of(table: &Table) -> Vec<Text> {
    table
        .items()
        .iter()
        .map(|item| Text::from_string(format!("{}@{}", item.key.as_str(), item.line.get())))
        .collect()
}

#[test]
fn entries_come_out_in_source_order_with_their_lines() {
    let table = table_of(parsed!("zeta = 1\nalpha = 2\n\n[mid]\nb = 1\na = 2\n"));
    assert_eq!(
        keys_of(&table),
        texts!["zeta@1", "alpha@2", "mid@4"],
        "source order, not sorted order"
    );
    assert_eq!(
        table.table_at(&Name::of("mid")).map(keys_of),
        Some(texts!["b@5", "a@6"]),
        "nested entries too"
    );
}

#[test]
fn headers_inline_tables_and_dotted_keys_build_the_same_tree() {
    let spellings = [
        ("header", "[lints.clippy]\nfoo = \"deny\"\n"),
        ("inline", "lints = { clippy = { foo = \"deny\" } }\n"),
        ("dotted", "lints.clippy.foo = \"deny\"\n"),
    ];
    for (label, text) in spellings {
        let table = table_of(parsed!(text));
        assert_eq!(
            table
                .table_at(&Name::of("lints"))
                .and_then(|lints| lints.table_at(&Name::of("clippy")))
                .and_then(|clippy| clippy.text_at(&Name::of("foo"))),
            Some(&Text::of("deny")),
            "{label} spelling reaches the same lint"
        );
    }
}

#[test]
fn arrays_of_tables_are_arrays_of_tables() {
    let table = table_of(parsed!(
        "[[package]]\nname = \"a\"\n\n[[package]]\nname = \"b\"\n"
    ));
    let items: &[Value] = match table.get(&Name::of("package")).map(|item| &item.value) {
        Some(Value::Array(found)) => found,
        Some(Value::Bool(_) | Value::Other | Value::Str(_) | Value::Table(_)) | None => &[],
    };
    let names: Vec<Option<&Text>> = items
        .iter()
        .map(|item| match item {
            Value::Table(inner) => inner.text_at(&Name::of("name")),
            Value::Array(_) | Value::Bool(_) | Value::Other | Value::Str(_) => None,
        })
        .collect();
    assert_eq!(
        names,
        [Some(&Text::of("a")), Some(&Text::of("b"))],
        "both packages, in order"
    );
}

#[test]
fn numbers_dates_and_floats_read_as_other_and_booleans_as_booleans() {
    let table = table_of(parsed!("i = 1\nf = 1.5\nd = 2024-05-01\nt = true\n"));
    for key in ["i", "f", "d"] {
        assert_eq!(
            table.get(&Name::of(key)).map(|item| &item.value),
            Some(&Value::Other),
            "{key} is not a string or table"
        );
    }
    assert_eq!(
        table.get(&Name::of("t")).map(|item| item.value.is_true()),
        Some(Answer::Yes),
        "true"
    );
    assert_eq!(
        table.get(&Name::of("i")).map(|item| item.value.is_true()),
        Some(Answer::No),
        "a number is not true"
    );
}

#[test]
fn multi_line_strings_and_comments_are_read() {
    let table = table_of(parsed!(
        "# lead\nname = \"\"\"\nline one\nline two\"\"\" # trailing\n"
    ));
    assert_eq!(
        table.text_at(&Name::of("name")),
        Some(&Text::of("line one\nline two")),
        "a multi-line string"
    );
}

#[test]
fn errors_name_line_and_column() {
    assert_eq!(
        message_of(parsed!("a = 1\nb = \n")).as_str(),
        "malformed TOML at line 2, column 5: string values must be quoted, expected literal string",
        "a missing value"
    );
    assert!(
        message_of(parsed!("a = 1\na = 2\n"))
            .as_str()
            .starts_with("malformed TOML at line 2, column 1:"),
        "a duplicate key"
    );
    assert_eq!(
        table_of(parsed!("")),
        Table::empty(),
        "an empty document is an empty table"
    );
}

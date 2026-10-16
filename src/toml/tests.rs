use super::{parse, Doc, Entry, KeyPath, Line, SectionKind, Value};
use crate::failure::Failure;
use crate::types::{Message, Name, RelPath, Source, Text};

macro_rules! parsed {
    ($text:expr_2021) => {
        parse(
            &RelPath::of("t.toml"),
            &Source::from_string($text.to_owned()),
        )
    };
}

macro_rules! texts {
    ($($item:expr_2021),* $(,)?) => {
        vec![$(Text::of($item)),*]
    };
}

/// The document of `result`; one that did not parse is empty, which fails the
/// assertions that follow.
fn doc_of(result: Result<Doc, Failure>) -> Doc {
    match result {
        Ok(found) => found,
        Err(_) => Doc {
            sections: Vec::new(),
        },
    }
}

/// The failure message of `result`, empty when it parsed.
fn message_of(result: Result<Doc, Failure>) -> Message {
    match result {
        Err(failure) => failure.text(),
        Ok(_) => Message::of(""),
    }
}

fn dotted(names: &[Name]) -> Text {
    Text::from_string(
        names
            .iter()
            .map(Name::as_str)
            .collect::<Vec<&str>>()
            .join("."),
    )
}

fn entry_shape(entry: &Entry) -> Text {
    Text::from_string(format!(
        "{}@{}",
        entry.key.dotted().as_str(),
        entry.line.get()
    ))
}

/// One line per section: kind, header, line, then its entries.
fn shape(doc: &Doc) -> Vec<Text> {
    doc.sections
        .iter()
        .map(|section| {
            Text::from_string(format!(
                "{:?} [{}] @{}: {}",
                section.kind,
                dotted(&section.header).as_str(),
                section.line.get(),
                section
                    .entries
                    .iter()
                    .map(|entry| entry_shape(entry).as_str().to_owned())
                    .collect::<Vec<String>>()
                    .join(" ")
            ))
        })
        .collect()
}

/// The first entry named `key` in any section.
#[must_use]
fn value_in(doc: &Doc, key: &Name) -> Option<Value> {
    doc.sections
        .iter()
        .flat_map(|section| section.entries.iter())
        .find(|entry| entry.key.dotted().as_str() == key.as_str())
        .map(|entry| entry.value.clone())
}

#[test]
fn sections_and_entries_keep_source_order_and_lines() {
    let text =
        "name = 1\n\n[a.b]\nx = 1\ny = 2\n\n[[arr]]\nn = 1\n\n[[arr]]\nn = 2\n\n[c]\nz = 3\n";
    assert_eq!(
        shape(&doc_of(parsed!(text))),
        texts![
            "Root [] @1: name@1",
            "Table [a.b] @3: x@4 y@5",
            "ArrayOfTables [arr] @7: n@8",
            "ArrayOfTables [arr] @10: n@11",
            "Table [c] @13: z@14",
        ],
        "one section per header, entries under their header"
    );
}

#[test]
fn a_header_on_the_first_line_follows_the_root_section() {
    assert_eq!(
        shape(&doc_of(parsed!("[a]\nx = 1\n"))),
        texts!["Root [] @1: ", "Table [a] @1: x@2"],
        "the root section always comes first"
    );
}

#[test]
fn dotted_keys_stay_dotted_entries_in_source_order() {
    assert_eq!(
        shape(&doc_of(parsed!("a.b = 1\na.c = 2\nd = 3\na.e = 4\n"))),
        texts!["Root [] @1: a.b@1 a.c@2 d@3 a.e@4"],
        "interleaved dotted keys keep the order they were written in"
    );
}

#[test]
fn implicit_parent_tables_make_no_section_and_no_entries() {
    assert_eq!(
        shape(&doc_of(parsed!("[a.b.c]\nk = 1\n"))),
        texts!["Root [] @1: ", "Table [a.b.c] @1: k@2"],
        "only the written header is a section"
    );
}

#[test]
fn a_header_below_a_dotted_key_table_keeps_its_full_path() {
    assert_eq!(
        shape(&doc_of(parsed!(
            "[fruit]\napple.color = \"red\"\n\n[fruit.apple.texture]\nsmooth = true\n"
        ))),
        texts![
            "Root [] @1: ",
            "Table [fruit] @1: apple.color@2",
            "Table [fruit.apple.texture] @4: smooth@5",
        ],
        "dotted keys belong to the enclosing section"
    );
}

#[test]
fn a_parent_header_after_its_child_is_still_found_by_name() {
    let doc = doc_of(parsed!("[a.b]\nx = 1\n[a]\ny = 2\n"));
    let wanted = [Name::of("a")];
    let names: Vec<&str> = doc
        .sections_named(&wanted, SectionKind::Table)
        .flat_map(|section| section.entries.iter())
        .filter_map(Entry::single_key)
        .map(Name::as_str)
        .collect();
    assert_eq!(names, ["y"], "the later parent header is its own section");
}

#[test]
fn quoted_and_spaced_keys_are_read() {
    assert_eq!(
        shape(&doc_of(parsed!("\"a.b\" = 1\n['x y' . z]\nk = 1\n"))),
        texts!["Root [] @1: a.b@1", "Table [x y.z] @2: k@3"],
        "a quoted key keeps its dot, a spaced header is split"
    );
}

#[test]
fn strings_integers_and_booleans_are_read() {
    let doc = doc_of(parsed!(
        "s = 'lit'\nb = \"a\\tb\\u00e9\"\ni = 1_000\nh = 0xff\nn = -5\nt = true\n"
    ));
    assert_eq!(
        value_in(&doc, &Name::of("s")),
        Some(Value::Str(Text::of("lit"))),
        "literal"
    );
    assert_eq!(
        value_in(&doc, &Name::of("b")),
        Some(Value::Str(Text::of("a\tb\u{e9}"))),
        "escapes"
    );
    assert_eq!(
        value_in(&doc, &Name::of("i")),
        Some(Value::Int(1000)),
        "underscore"
    );
    assert_eq!(value_in(&doc, &Name::of("h")), Some(Value::Int(255)), "hex");
    assert_eq!(
        value_in(&doc, &Name::of("n")),
        Some(Value::Int(-5)),
        "negative"
    );
    assert_eq!(
        value_in(&doc, &Name::of("t")),
        Some(Value::Bool(true)),
        "boolean"
    );
}

#[test]
fn multi_line_strings_floats_and_dates_now_parse() {
    let doc = doc_of(parsed!(
        "m = \"\"\"\nfirst\nsecond\"\"\"\nl = '''\nraw \\n'''\nf = 1.5e2\nd = 2024-05-01\nt = 07:32:00\n"
    ));
    assert_eq!(
        value_in(&doc, &Name::of("m")),
        Some(Value::Str(Text::of("first\nsecond"))),
        "a multi-line basic string drops the first newline"
    );
    assert_eq!(
        value_in(&doc, &Name::of("l")),
        Some(Value::Str(Text::of("raw \\n"))),
        "a multi-line literal string keeps backslashes"
    );
    assert_eq!(
        value_in(&doc, &Name::of("f")),
        Some(Value::Raw(Text::of("1.5e2"))),
        "a float is kept as text"
    );
    assert_eq!(
        value_in(&doc, &Name::of("d")),
        Some(Value::Raw(Text::of("2024-05-01"))),
        "a date is kept as text"
    );
    assert_eq!(
        value_in(&doc, &Name::of("t")),
        Some(Value::Raw(Text::of("07:32:00"))),
        "a time is kept as text"
    );
}

#[test]
fn inline_tables_nest_deeply_and_may_span_lines() {
    let doc = doc_of(parsed!(
        "x = { a = { b = { c = { d = 1 } } }, e.f = 2,\n  g = [1, { h = 3 }],\n}\n"
    ));
    let entry_of = |name: &str, below: Value| Entry {
        key: KeyPath::of(Name::of(name)),
        line: Line::new(1),
        value: below,
    };
    let innermost = Value::Table(vec![entry_of("d", Value::Int(1))]);
    let deep = Value::Table(vec![entry_of(
        "b",
        Value::Table(vec![entry_of("c", innermost)]),
    )]);
    let found = value_in(&doc, &Name::of("x"));
    let entries: &[Entry] = match &found {
        Some(Value::Table(items)) => items,
        Some(Value::Array(_) | Value::Bool(_) | Value::Int(_) | Value::Raw(_) | Value::Str(_))
        | None => &[],
    };
    assert_eq!(
        entries.iter().map(entry_shape).collect::<Vec<Text>>(),
        texts!["a@1", "e.f@1", "g@2"],
        "entries in source order with dotted keys and their lines"
    );
    assert_eq!(
        entries.first().map(|first| first.value.clone()),
        Some(deep),
        "four levels of inline tables"
    );
}

#[test]
fn arrays_may_span_lines_hold_comments_and_end_with_a_comma() {
    let doc = doc_of(parsed!("a = [\n  1, # one\n  2,\n]\n"));
    assert_eq!(
        value_in(&doc, &Name::of("a")),
        Some(Value::Array(vec![Value::Int(1), Value::Int(2)])),
        "multi-line array"
    );
}

#[test]
fn syntax_errors_name_file_line_and_column() {
    assert_eq!(
        message_of(parsed!("a = 1\nb = \n")).as_str(),
        "t.toml:2:5: malformed TOML: string values must be quoted, expected literal string",
        "a missing value"
    );
    assert!(
        message_of(parsed!("a = 1\na = 2\n"))
            .as_str()
            .starts_with("t.toml:2:1: malformed TOML: duplicate key"),
        "a duplicate key names the second one"
    );
    assert!(
        message_of(parsed!("[x\n"))
            .as_str()
            .starts_with("t.toml:1:3: malformed TOML: unclosed table"),
        "an unclosed header"
    );
}

#[test]
fn a_column_counts_characters_not_bytes() {
    let message = message_of(parsed!("\"\u{e9}\u{e9}\" = [1,, 2]\n"));
    assert!(
        message.as_str().starts_with("t.toml:1:11:"),
        "two two-byte characters before the error count as two columns: {}",
        message.as_str()
    );
}

#[test]
fn an_integer_beyond_sixty_four_bits_is_kept_as_text() {
    let doc = doc_of(parsed!("ok = 1\nbig = 99999999999999999999\n"));
    let values: Vec<Value> = doc
        .sections
        .iter()
        .flat_map(|section| section.entries.iter())
        .map(|entry| entry.value.clone())
        .collect();
    assert_eq!(
        values,
        vec![Value::Int(1), Value::Raw(Text::of("99999999999999999999"))],
        "the file parses; only the reader of that key can refuse it"
    );
}

#[test]
fn an_array_of_arrays_is_a_plain_entry_not_an_array_of_tables() {
    let doc = doc_of(parsed!("foo = [[[\"x\"]]]\n"));
    let entries: Vec<&Entry> = doc
        .sections
        .iter()
        .flat_map(|section| section.entries.iter())
        .collect();
    assert_eq!(
        entries.first().map(|entry| entry.value.clone()),
        Some(Value::Array(vec![Value::Array(vec![Value::Array(vec![
            Value::Str(Text::of("x"))
        ])])])),
        "the nested array is the value of `foo`"
    );
    assert_eq!(
        doc.sections
            .iter()
            .filter(|section| section.kind == SectionKind::ArrayOfTables)
            .count(),
        0,
        "no array-of-tables section is invented"
    );
}

#[test]
fn dotted_keys_with_array_and_inline_table_values_stay_entries_of_their_section() {
    let doc = doc_of(parsed!("[s]\na.b = [1]\nc.d = {x=1}\n"));
    let section = doc
        .sections
        .iter()
        .find(|candidate| candidate.kind == SectionKind::Table);
    let seen: Vec<(Text, Value)> = match section {
        Some(found) => found
            .entries
            .iter()
            .map(|entry| (entry.key.dotted(), entry.value.clone()))
            .collect(),
        None => Vec::new(),
    };
    assert_eq!(
        seen,
        vec![
            (Text::of("a.b"), Value::Array(vec![Value::Int(1)])),
            (
                Text::of("c.d"),
                Value::Table(vec![Entry {
                    key: KeyPath::of(Name::of("x")),
                    line: Line::new(3),
                    value: Value::Int(1),
                }])
            ),
        ],
        "both keep their dotted names and values"
    );
}

#[test]
fn a_table_defined_twice_is_refused() {
    let message = message_of(parsed!("[a]\nx = 1\n[a]\ny = 2\n"));
    assert!(
        message.as_str().starts_with("t.toml:3:2:"),
        "a repeated header is an error: {}",
        message.as_str()
    );
}

#[test]
fn an_empty_key_path_cannot_be_built() {
    assert_eq!(
        KeyPath::from_segments(&[]),
        None,
        "a TOML key has at least one segment"
    );
}

#[test]
fn a_key_path_built_from_segments_keeps_them() {
    let names = [Name::of("a"), Name::of("b"), Name::of("c")];
    let path = KeyPath::from_segments(&names);
    assert_eq!(
        path.as_ref().map(KeyPath::dotted),
        Some(Text::of("a.b.c")),
        "dotted joins the segments"
    );
    assert_eq!(
        path.as_ref()
            .map(|found| found.iter().cloned().collect::<Vec<Name>>()),
        Some(names.to_vec()),
        "iter yields the segments in order"
    );
    assert_eq!(
        path.as_ref().and_then(KeyPath::single),
        None,
        "three segments are not a single key"
    );
    assert_eq!(
        path.as_ref().and_then(KeyPath::without_first),
        KeyPath::from_segments(&names[1..]),
        "without_first drops the first segment"
    );
}

#[test]
fn a_single_key_round_trips() {
    let path = KeyPath::of(Name::of("only"));
    assert_eq!(path.single(), Some(&Name::of("only")), "one segment");
    assert_eq!(path.pair(), None, "one segment is not a pair");
    assert_eq!(
        path.without_first(),
        None,
        "nothing is left after the first"
    );
    assert_eq!(
        KeyPath::from_segments(&[Name::of("only")]),
        Some(path),
        "from_segments agrees with of"
    );
}

#[test]
fn a_dotted_key_round_trips_through_the_reader() {
    let doc = doc_of(parsed!("a.b = 1\nc = 2\n"));
    let keys: Vec<KeyPath> = doc
        .sections
        .iter()
        .flat_map(|section| section.entries.iter())
        .map(|entry| entry.key.clone())
        .collect();
    assert_eq!(
        keys,
        vec![
            KeyPath {
                first: Name::of("a"),
                rest: vec![Name::of("b")],
            },
            KeyPath::of(Name::of("c")),
        ],
        "a dotted key keeps both segments and a plain key has none after the first"
    );
    assert_eq!(
        keys.first().and_then(KeyPath::pair),
        Some((&Name::of("a"), &Name::of("b"))),
        "a dotted key of two segments is a pair"
    );
    assert_eq!(
        keys.iter().map(KeyPath::dotted).collect::<Vec<Text>>(),
        texts!["a.b", "c"],
        "dotted reproduces the written keys"
    );
}

#[test]
fn extending_a_key_path_appends_one_segment() {
    let base = KeyPath::of(Name::of("a"));
    assert_eq!(
        KeyPath::extending(None, &Name::of("a")),
        base,
        "no prefix gives a single key"
    );
    assert_eq!(
        KeyPath::extending(Some(&base), &Name::of("b")).dotted(),
        Text::of("a.b"),
        "a prefix is followed by the new segment"
    );
}

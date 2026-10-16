//! TOML reading through the `toml` crate: syntax the earlier subset reader
//! refused (multi-line strings, floats, dates, deeply nested inline tables)
//! now parses, and a syntax error names file, line and column.
use crate::support::{Exit, Fail, Flag, Message, Name, Outcome, RelPath, Sandbox};

/// A manifest table using every construct the old reader refused.
const NEWLY_SUPPORTED: &str = "
[package.metadata.extra]
ratio = 1.5
shifted = -3.0e-2
released = 2024-05-01
stamp = 2024-05-01T07:32:00Z
nested = { a = { b = { c = { d = [1, 2, { e = 3 }] } } } }
notes = \"\"\"
first line
second line
\"\"\"
literal = '''
raw \\n text
'''
hex = 0xDEAD_BEEF
";

fn prepared(name: &Name, config: &Message) -> Result<Sandbox, Fail> {
    let package = Name::of("fx");
    let sandbox = Sandbox::create(name)?;
    sandbox.seed(&package, config)?;
    sandbox.write(&RelPath::of("src/main.rs"), &Message::of("fn main() {}\n"))?;
    Ok(sandbox)
}

fn run_committed(sandbox: &Sandbox) -> Result<Outcome, Fail> {
    sandbox.init_repo()?;
    sandbox.run(&[Flag::Range(Message::of("HEAD"))])
}

#[test]
fn a_manifest_may_use_multi_line_strings_floats_dates_and_deep_inline_tables() {
    assert_eq!(
        check_a_manifest_may_use_newly_supported_syntax(),
        Ok(()),
        "a_manifest_may_use_multi_line_strings_floats_dates_and_deep_inline_tables"
    );
}

fn check_a_manifest_may_use_newly_supported_syntax() -> Result<(), Fail> {
    let package = Name::of("fx");
    let sandbox = prepared(
        &Name::of("toml-newly-supported"),
        &Sandbox::default_config(&package),
    )?;
    sandbox.append(&RelPath::of("Cargo.toml"), &Message::of(NEWLY_SUPPORTED))?;
    let outcome = run_committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Clean,
        "the manifest is read in full: {:?} {:?}",
        outcome.stderr(),
        outcome.findings()
    );
    Ok(())
}

#[test]
fn a_config_may_use_multi_line_literals_comments_and_trailing_commas() {
    assert_eq!(
        check_a_config_may_use_toml_one_point_one_syntax(),
        Ok(()),
        "a_config_may_use_multi_line_literals_comments_and_trailing_commas"
    );
}

fn check_a_config_may_use_toml_one_point_one_syntax() -> Result<(), Fail> {
    let config = Message::of(
        "baseline = \"forbidden\"
default-range = '''HEAD'''
code = [\"src/**\"]
judge = [
  # a comment inside the array
  \"\"\"docs/**\"\"\",
  'ci/**',
]

[member.fx]
role = \"pure\"
ambient = [
  { path = \"std::fs\",
    files = [\"src/**\"],
  },
]
",
    );
    let sandbox = prepared(&Name::of("toml-config-syntax"), &config)?;
    let outcome = run_committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Clean,
        "the config is read in full: {:?} {:?}",
        outcome.stderr(),
        outcome.findings()
    );
    Ok(())
}

#[test]
fn a_toml_syntax_error_names_file_line_and_column() {
    assert_eq!(
        check_a_toml_syntax_error_names_file_line_and_column(),
        Ok(()),
        "a_toml_syntax_error_names_file_line_and_column"
    );
}

fn check_a_toml_syntax_error_names_file_line_and_column() -> Result<(), Fail> {
    let config = Message::of("baseline = \"forbidden\"\ndefault-range = [\"HEAD\"\nrole = 1\n");
    let sandbox = prepared(&Name::of("toml-error-position"), &config)?;
    let outcome = run_committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Error, "a syntax error exits 2");
    ensure!(
        outcome
            .stderr()
            .as_str()
            .contains(".unrepresentable/config:3:"),
        "the error names file and line: {:?}",
        outcome.stderr()
    );
    ensure!(
        outcome.stderr().as_str().contains("malformed TOML"),
        "the error says what failed: {:?}",
        outcome.stderr()
    );
    Ok(())
}

#[test]
fn a_duplicate_key_is_a_parse_error() {
    assert_eq!(
        check_a_duplicate_key_is_a_parse_error(),
        Ok(()),
        "a_duplicate_key_is_a_parse_error"
    );
}

fn check_a_duplicate_key_is_a_parse_error() -> Result<(), Fail> {
    let config = Message::of(
        "baseline = \"forbidden\"\ndefault-range = \"HEAD\"\ndefault-range = \"HEAD\"\n\n[member.fx]\nrole = \"pure\"\n",
    );
    let sandbox = prepared(&Name::of("toml-duplicate-key"), &config)?;
    let outcome = run_committed(&sandbox)?;
    ensure_eq!(outcome.exit, Exit::Error, "a duplicate key exits 2");
    ensure!(
        outcome
            .stderr()
            .as_str()
            .contains(".unrepresentable/config:3:1: malformed TOML: duplicate key"),
        "the error names the second key: {:?}",
        outcome.stderr()
    );
    Ok(())
}

#[test]
fn a_config_count_beyond_sixty_four_bits_is_refused_with_its_position() {
    assert_eq!(
        check_a_config_count_beyond_sixty_four_bits_is_refused_with_its_position(),
        Ok(()),
        "a_config_count_beyond_sixty_four_bits_is_refused_with_its_position"
    );
}

fn check_a_config_count_beyond_sixty_four_bits_is_refused_with_its_position() -> Result<(), Fail> {
    let package = Name::of("fx");
    let mut config = Sandbox::default_config(&package).as_str().to_owned();
    config.push_str(
        "\n[[macro-comment]]\nmacro = \"m\"\nprefix = \"// \"\nwithin-lines = 99999999999999999999\n",
    );
    let sandbox = prepared(&Name::of("toml-huge-count"), &Message::from_string(config))?;
    let outcome = run_committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Error,
        "a reader that needs an integer rejects it"
    );
    ensure!(
        outcome
            .stderr()
            .as_str()
            .contains(".unrepresentable/config:"),
        "the error names the config and a line: {:?}",
        outcome.stderr()
    );
    Ok(())
}

#[test]
fn a_nested_array_cannot_hide_an_unknown_config_key() {
    assert_eq!(
        check_a_nested_array_cannot_hide_an_unknown_config_key(),
        Ok(()),
        "a_nested_array_cannot_hide_an_unknown_config_key"
    );
}

fn check_a_nested_array_cannot_hide_an_unknown_config_key() -> Result<(), Fail> {
    let package = Name::of("fx");
    let mut config = String::from("bogus = [[[\"x\"]]]\n");
    config.push_str(Sandbox::default_config(&package).as_str());
    let sandbox = prepared(
        &Name::of("toml-nested-array"),
        &Message::from_string(config),
    )?;
    let outcome = run_committed(&sandbox)?;
    ensure_eq!(
        outcome.exit,
        Exit::Error,
        "an unknown key is rejected whatever its value looks like: {:?}",
        outcome.stdout()
    );
    ensure!(
        outcome
            .stderr()
            .as_str()
            .contains("unknown config key `bogus`"),
        "the key is named: {:?}",
        outcome.stderr()
    );
    Ok(())
}

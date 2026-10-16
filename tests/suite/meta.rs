//! Meta-test: every rule id declared in `src/rules.rs` has a positive and a
//! negative case in this suite, file-level and config-level rules alike.
//!
//! A fixture case (`check_case`) counts only when its expected findings agree:
//! the positive one names the rule, the negative one does not. Any other test
//! must name the rule (positive) or assert the absence of findings (negative).
//!
//! Config-level rules (`manifest-lints`, `stray-lint-config`, `lint-coverage`,
//! `forbidden-dependency`, `forbidden-reference`, `unreached-file`,
//! `judge-code-pairing`, `baseline`, `canary`, `preference-omission`) need a whole
//! repository, so a `//~` mark cannot express them; the sandbox tests of this
//! suite cover them. The table lists exactly the rules of `src/rules.rs`, in
//! order.
use std::fs;

use std::path::{Path, PathBuf};

use crate::support::{Fail, Message, Name};

macro_rules! coverage {
    ($rule:literal, $positive:literal, $negative:literal) => {
        Coverage {
            rule: Name::of($rule),
            positive: Name::of($positive),
            negative: Name::of($negative),
        }
    };
}

/// The cases that cover one rule.
struct Coverage {
    negative: Name,
    positive: Name,
    rule: Name,
}

/// Which half of a rule's coverage is being checked.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Half {
    Negative,
    Positive,
}

fn file_level_rows() -> Vec<Coverage> {
    vec![
        coverage!(
            "abort-unwrap",
            "abort_unwrap_positive",
            "abort_unwrap_negative"
        ),
        coverage!(
            "abort-panic",
            "abort_panic_positive",
            "abort_panic_negative"
        ),
        coverage!(
            "abort-assert",
            "abort_assert_positive",
            "abort_assert_negative"
        ),
        coverage!(
            "silent-fallback",
            "silent_fallback_positive",
            "silent_fallback_negative"
        ),
        coverage!(
            "discarded-value",
            "discarded_value_positive",
            "discarded_value_negative"
        ),
        coverage!(
            "pub-fn-name",
            "pub_fn_name_positive",
            "pub_fn_name_negative"
        ),
        coverage!(
            "primitive-field",
            "primitive_field_positive",
            "primitive_field_negative"
        ),
        coverage!(
            "primitive-signature",
            "primitive_signature_positive",
            "primitive_signature_negative"
        ),
    ]
}

fn syntax_rows() -> Vec<Coverage> {
    vec![
        coverage!("bool-param", "bool_param_positive", "bool_param_negative"),
        coverage!(
            "transposable-params",
            "transposable_params_positive",
            "transposable_params_negative"
        ),
        coverage!(
            "option-not-must-use",
            "option_not_must_use_positive",
            "option_not_must_use_negative"
        ),
        coverage!(
            "modular-arith",
            "modular_arith_positive",
            "modular_arith_negative"
        ),
        coverage!(
            "arith-operator",
            "arith_operator_positive",
            "arith_operator_negative"
        ),
    ]
}

fn fixture_rows() -> Vec<Coverage> {
    vec![
        coverage!(
            "suppression",
            "suppression_positive",
            "suppression_negative"
        ),
        coverage!(
            "unordered-collection",
            "unordered_collection_positive",
            "unordered_collection_negative"
        ),
        coverage!(
            "memory-leak",
            "memory_leak_positive",
            "memory_leak_negative"
        ),
        coverage!("float-type", "float_type_positive", "float_type_negative"),
        coverage!(
            "ambient-input",
            "ambient_input_positive",
            "ambient_input_negative"
        ),
        coverage!(
            "numeric-cast",
            "numeric_cast_positive",
            "numeric_cast_negative"
        ),
    ]
}

fn pattern_rows() -> Vec<Coverage> {
    vec![
        coverage!(
            "catch-all-arm",
            "catch_all_arm_wildcard",
            "catch_all_arm_named_variants"
        ),
        coverage!(
            "refutable-shortcut",
            "refutable_shortcut_if_let",
            "refutable_shortcut_negative"
        ),
        coverage!(
            "struct-rest-pattern",
            "struct_rest_pattern_positive",
            "struct_rest_pattern_negative"
        ),
        coverage!(
            "default-construction",
            "default_construction_positive",
            "default_construction_negative"
        ),
    ]
}

fn discard_rows() -> Vec<Coverage> {
    vec![
        coverage!(
            "unused-underscore-binding",
            "unused_underscore_binding_positive",
            "unused_underscore_binding_negative"
        ),
        coverage!(
            "dropped-call-result",
            "dropped_call_result_positive",
            "dropped_call_result_negative"
        ),
        coverage!(
            "swallowed-error",
            "swallowed_error_positive",
            "swallowed_error_negative"
        ),
    ]
}

fn tail_rows() -> Vec<Coverage> {
    vec![
        coverage!("cfg-clippy", "cfg_clippy_positive", "cfg_clippy_negative"),
        coverage!("banned", "banned_direct_and_macro", "banned_allowed_path"),
        coverage!(
            "test-assertion",
            "case_test_assertion_no_assertion",
            "case_test_assertion_negative"
        ),
        coverage!(
            "test-early-return",
            "case_test_early_return_positive",
            "case_test_early_return_negative"
        ),
        coverage!(
            "macro-needs-comment",
            "macro_needs_comment_positive",
            "macro_needs_comment_negative"
        ),
        coverage!(
            "test-attributes",
            "case_test_attributes_positive",
            "case_test_attributes_negative"
        ),
        coverage!(
            "manifest-lints",
            "manifest_lints_positive",
            "manifest_lints_negative"
        ),
    ]
}

fn config_level_rows() -> Vec<Coverage> {
    vec![
        coverage!(
            "stray-lint-config",
            "stray_lint_config_cargo_rustflags",
            "stray_lint_config_harmless_cargo_config"
        ),
        coverage!(
            "lint-coverage",
            "lint_coverage_reports_a_removed_lint",
            "lint_coverage_negative"
        ),
        coverage!(
            "forbidden-dependency",
            "forbidden_dependency_positive",
            "forbidden_dependency_negative"
        ),
        coverage!(
            "forbidden-reference",
            "forbidden_reference_positive",
            "forbidden_reference_negative"
        ),
        coverage!(
            "unreached-file",
            "unreached_file_positive",
            "unreached_file_negative"
        ),
        coverage!(
            "unscannable-include",
            "unscannable_include_non_literal",
            "unscannable_include_literal"
        ),
        coverage!(
            "unparsed-macro",
            "unparsed_macro_positive",
            "unparsed_macro_negative"
        ),
        coverage!(
            "preference-omission",
            "preference_omission_positive",
            "preference_omission_negative"
        ),
        coverage!(
            "baseline",
            "baseline_stale_entry",
            "baseline_shrink_only_covered"
        ),
        coverage!("canary", "canary_positive", "canary_negative"),
        coverage!(
            "judge-code-pairing",
            "judge_and_code_in_one_change_is_a_finding",
            "a_new_fixture_beside_a_code_change_is_free"
        ),
    ]
}

/// One row per rule id, in the order of `src/rules.rs`.
/// One row per rule id, in the order of `src/rules.rs`.
fn table() -> Vec<Coverage> {
    let mut rows = file_level_rows();
    rows.extend(syntax_rows());
    rows.extend(fixture_rows());
    rows.extend(pattern_rows());
    rows.extend(discard_rows());
    rows.extend(tail_rows());
    rows.extend(config_level_rows());
    rows
}

fn repo_path(relative: &Message) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative.as_str())
}

/// The rule ids `src/rules.rs` declares.
fn declared_rules() -> Result<Vec<Name>, Fail> {
    let source = fs::read_to_string(repo_path(&Message::of("src/rules.rs")))?;
    Ok(source
        .lines()
        .filter_map(|line| line.split_once("RuleId(\"").map(|(_, rest)| rest))
        .filter_map(|rest| rest.split_once('"').map(|(id, _)| Name::of(id)))
        .collect())
}

/// Every test module of the suite except this one, concatenated.
fn suite_source() -> Result<Message, Fail> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(repo_path(&Message::of("tests/suite")))? {
        let path = entry?.path();
        if path.file_name().and_then(|name| name.to_str()) != Some("meta.rs") {
            paths.push(path);
        }
    }
    paths.sort();
    let mut all = String::new();
    for path in paths {
        all.push_str(&fs::read_to_string(path)?);
        all.push('\n');
    }
    Ok(Message::from_string(all))
}

/// The text of test `name`, up to the next `#[test]`; its `check_` helper follows it.
#[must_use]
fn test_body(source: &Message, name: &Name) -> Option<Message> {
    let marker = format!("fn {}(", name.as_str());
    let (_, rest) = source.as_str().split_once(&marker)?;
    let body = rest.split("\n#[test]").next()?;
    Some(Message::of(body))
}

/// The case a `check_case` fixture test runs, if it is one.
#[must_use]
fn fixture_case(body: &Message) -> Option<Name> {
    let (_, rest) = body.as_str().split_once("check_case(&Name::of(\"")?;
    let (case, _) = rest.split_once('"')?;
    Some(Name::of(case))
}

/// The expected findings of `case` that name `rule`.
fn expected_findings_of(case: &Name, entry: &Coverage) -> Result<Vec<Message>, Fail> {
    let path = repo_path(&Message::from_string(format!(
        "tests/expected/{}.txt",
        case.as_str()
    )));
    let text = fs::read_to_string(path)?;
    let rule = &entry.rule;
    let prefix = format!("{}:", rule.as_str());
    Ok(text
        .lines()
        .filter(|line| {
            line.rsplit_once(": ")
                .is_some_and(|(_, found)| found == rule.as_str() || found.starts_with(&prefix))
        })
        .map(Message::of)
        .collect())
}

/// What is wrong with one half of one rule's coverage; empty when nothing is.
fn problems(entry: &Coverage, source: &Message, half: Half) -> Result<Vec<Message>, Fail> {
    let name = match half {
        Half::Positive => &entry.positive,
        Half::Negative => &entry.negative,
    };
    let Some(body) = test_body(source, name) else {
        return Ok(vec![Message::from_string(format!(
            "`{}`: there is no test `{}`",
            entry.rule.as_str(),
            name.as_str()
        ))]);
    };
    let named = body
        .as_str()
        .contains(&format!("\"{}\"", entry.rule.as_str()));
    let silent = body.as_str().contains("Vec::<Message>::new()")
        || body.as_str().contains("Answer::No")
        || body.as_str().contains("Exit::Clean");
    let from_expected = match fixture_case(&body) {
        Some(case) => Some(expected_findings_of(&case, entry)?),
        None => None,
    };
    let fine = match (half, from_expected) {
        (Half::Positive, Some(lines)) => !lines.is_empty(),
        (Half::Negative, Some(lines)) => lines.is_empty(),
        (Half::Positive, None) => named,
        (Half::Negative, None) => named || silent,
    };
    if fine {
        Ok(Vec::new())
    } else {
        Ok(vec![Message::from_string(format!(
            "`{}`: test `{}` is not a {} case for the rule",
            entry.rule.as_str(),
            name.as_str(),
            match half {
                Half::Positive => "positive",
                Half::Negative => "negative",
            }
        ))])
    }
}

#[test]
fn every_rule_has_a_positive_and_a_negative_case() {
    assert_eq!(
        check_every_rule_has_a_positive_and_a_negative_case(),
        Ok(()),
        "every_rule_has_a_positive_and_a_negative_case"
    );
}

fn check_every_rule_has_a_positive_and_a_negative_case() -> Result<(), Fail> {
    let rows = table();
    let declared = declared_rules()?;
    let listed: Vec<Name> = rows.iter().map(|row| row.rule.clone()).collect();
    ensure_eq!(
        listed,
        declared,
        "the coverage table lists exactly the rules of src/rules.rs, in order"
    );
    let source = suite_source()?;
    let mut found: Vec<Message> = Vec::new();
    for row in &rows {
        ensure!(
            row.positive != row.negative,
            "`{}` needs two different cases",
            row.rule.as_str()
        );
        found.extend(problems(row, &source, Half::Positive)?);
        found.extend(problems(row, &source, Half::Negative)?);
    }
    ensure_eq!(
        found,
        Vec::<Message>::new(),
        "every rule has a positive and a negative case"
    );
    Ok(())
}

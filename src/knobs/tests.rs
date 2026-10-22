use super::{at_least_as_strict, find, Knob, Scale, Setting, Transition};
use crate::toml::Value;
use crate::types::{Answer, Holds as _, Name, Text};

/// Is the change from `$old` to `$new` (each `Some(number)` or `None` for an
/// absent knob) of the knob `$name` at least as strict? `None` for an unknown knob.
macro_rules! answer {
    ($name:expr_2021, $old:expr_2021, $new:expr_2021) => {{
        let old_value = $old.map(Value::Int);
        let new_value = $new.map(Value::Int);
        answer_for(
            &Name::of($name),
            &Transition {
                new: new_value.as_ref(),
                old: old_value.as_ref(),
            },
        )
    }};
}

/// Like `answer!`, for any kind of TOML value: `$old` and `$new` are each
/// `Some(Value)` or `None` for an absent knob.
macro_rules! answer_value {
    ($name:expr_2021, $old:expr_2021, $new:expr_2021) => {{
        let old_value: Option<Value> = $old;
        let new_value: Option<Value> = $new;
        answer_for(
            &Name::of($name),
            &Transition {
                new: new_value.as_ref(),
                old: old_value.as_ref(),
            },
        )
    }};
}

/// Is `change` to the knob called `name` at least as strict? `None` if the knob is unknown.
#[must_use]
fn answer_for(name: &Name, change: &Transition<'_>) -> Option<Answer> {
    find(name).map(|knob| at_least_as_strict(knob, change))
}

/// The scale of the knob called `name`, if the table has it.
#[must_use]
fn scale_of(name: &Name) -> Option<Scale> {
    find(name).map(|knob| knob.scale)
}

#[test]
fn size_bound_knobs_are_on_the_higher_scale() {
    assert_eq!(
        scale_of(&Name::of("too-large-for-stack")),
        Some(Scale::Higher)
    );
    assert_eq!(
        scale_of(&Name::of("unnecessary-box-size")),
        Some(Scale::Higher)
    );
    assert_eq!(
        scale_of(&Name::of("vec-box-size-threshold")),
        Some(Scale::Higher)
    );
}

#[test]
fn count_and_limit_knobs_are_on_the_lower_scale() {
    assert_eq!(
        scale_of(&Name::of("single-char-binding-names-threshold")),
        Some(Scale::Lower)
    );
    assert_eq!(
        scale_of(&Name::of("too-many-lines-threshold")),
        Some(Scale::Lower)
    );
    assert_eq!(
        scale_of(&Name::of("excessive-nesting-threshold")),
        Some(Scale::LowerZeroOff)
    );
}

/// Is the strictest value of `knob` an integer above its default?
#[must_use]
fn strictest_above_default(knob: &Knob) -> Answer {
    match (knob.default, knob.strictest) {
        (Setting::Int(default), Setting::Int(strictest)) => (strictest > default).answer(),
        (
            Setting::Bool(_) | Setting::Empty | Setting::Listed | Setting::Str(_),
            Setting::Bool(_) | Setting::Empty | Setting::Int(_) | Setting::Listed | Setting::Str(_),
        )
        | (
            Setting::Int(_),
            Setting::Bool(_) | Setting::Empty | Setting::Listed | Setting::Str(_),
        ) => Answer::No,
    }
}

#[test]
fn the_strictest_value_of_a_higher_knob_is_above_its_default() {
    for name in [
        "too-large-for-stack",
        "unnecessary-box-size",
        "vec-box-size-threshold",
    ] {
        assert_eq!(
            find(&Name::of(name)).map(strictest_above_default),
            Some(Answer::Yes),
            "{name}"
        );
    }
}

#[test]
fn higher_knob_raising_is_at_least_as_strict() {
    assert_eq!(
        answer!("too-large-for-stack", Some(200), Some(1024)),
        Some(Answer::Yes)
    );
    assert_eq!(
        answer!("unnecessary-box-size", Some(128), Some(512)),
        Some(Answer::Yes)
    );
    assert_eq!(
        answer!("vec-box-size-threshold", Some(4096), Some(0x4000)),
        Some(Answer::Yes)
    );
}

#[test]
fn higher_knob_lowering_is_not_at_least_as_strict() {
    assert_eq!(
        answer!("too-large-for-stack", Some(1024), Some(200)),
        Some(Answer::No)
    );
    assert_eq!(
        answer!("unnecessary-box-size", Some(512), Some(128)),
        Some(Answer::No)
    );
    assert_eq!(
        answer!("vec-box-size-threshold", Some(0x4000), Some(4096)),
        Some(Answer::No)
    );
}

#[test]
fn higher_knob_equal_value_is_at_least_as_strict() {
    assert_eq!(
        answer!("too-large-for-stack", Some(200), Some(200)),
        Some(Answer::Yes)
    );
}

#[test]
fn higher_knob_zero_and_negative_values_are_lowerings() {
    assert_eq!(
        answer!("too-large-for-stack", Some(1024), Some(0)),
        Some(Answer::No)
    );
    assert_eq!(
        answer!("too-large-for-stack", Some(1024), Some(-1)),
        Some(Answer::No)
    );
}

#[test]
fn higher_knob_above_the_strictest_value_is_at_least_as_strict() {
    assert_eq!(
        answer!("too-large-for-stack", Some(1024), Some(4096)),
        Some(Answer::Yes)
    );
}

#[test]
fn removing_a_higher_knob_falls_back_to_a_looser_default() {
    assert_eq!(
        answer!("too-large-for-stack", Some(1024), None::<i64>),
        Some(Answer::No)
    );
    assert_eq!(
        answer!("vec-box-size-threshold", Some(0x4000), None::<i64>),
        Some(Answer::No)
    );
}

#[test]
fn adding_a_higher_knob_is_strict_only_above_the_default() {
    assert_eq!(
        answer!("too-large-for-stack", None::<i64>, Some(1024)),
        Some(Answer::Yes)
    );
    assert_eq!(
        answer!("too-large-for-stack", None::<i64>, Some(64)),
        Some(Answer::No)
    );
}

#[test]
fn lower_knob_lowering_is_at_least_as_strict() {
    assert_eq!(
        answer!("too-many-lines-threshold", Some(100), Some(60)),
        Some(Answer::Yes)
    );
}

#[test]
fn lower_knob_raising_is_not_at_least_as_strict() {
    assert_eq!(
        answer!("too-many-lines-threshold", Some(60), Some(100)),
        Some(Answer::No)
    );
}

#[test]
fn single_char_knob_reaching_zero_is_stricter() {
    assert_eq!(
        answer!("single-char-binding-names-threshold", Some(1), Some(0)),
        Some(Answer::Yes)
    );
    assert_eq!(
        answer!("single-char-binding-names-threshold", Some(0), Some(1)),
        Some(Answer::No)
    );
}

#[test]
fn switching_a_zero_off_knob_off_is_a_loosening() {
    assert_eq!(
        answer!("excessive-nesting-threshold", Some(5), Some(0)),
        Some(Answer::No)
    );
    assert_eq!(
        answer!("excessive-nesting-threshold", Some(0), Some(5)),
        Some(Answer::Yes)
    );
}

#[test]
fn an_exact_knob_is_stricter_only_when_it_reaches_the_strictest_value() {
    assert_eq!(
        answer_value!(
            "avoid-breaking-exported-api",
            Some(Value::Bool(true)),
            Some(Value::Bool(false))
        ),
        Some(Answer::Yes)
    );
    assert_eq!(
        answer_value!(
            "avoid-breaking-exported-api",
            Some(Value::Bool(false)),
            Some(Value::Bool(true))
        ),
        Some(Answer::No)
    );
    assert_eq!(
        answer_value!(
            "check-grouped-late-init",
            Some(Value::Bool(false)),
            Some(Value::Bool(true))
        ),
        Some(Answer::Yes)
    );
}

#[test]
fn a_non_integer_value_of_an_integer_knob_is_stricter_only_at_the_strictest_value() {
    assert_eq!(
        answer_value!(
            "too-many-lines-threshold",
            Some(Value::Int(100)),
            Some(Value::Bool(true))
        ),
        Some(Answer::No)
    );
    assert_eq!(
        answer_value!(
            "too-many-lines-threshold",
            Some(Value::Bool(true)),
            Some(Value::Int(60))
        ),
        Some(Answer::Yes)
    );
    assert_eq!(
        answer_value!(
            "too-many-lines-threshold",
            Some(Value::Bool(true)),
            Some(Value::Int(100))
        ),
        Some(Answer::No)
    );
    assert_eq!(
        answer_value!(
            "too-large-for-stack",
            Some(Value::Int(200)),
            Some(Value::Bool(true))
        ),
        Some(Answer::No)
    );
    assert_eq!(
        answer_value!(
            "too-large-for-stack",
            Some(Value::Bool(true)),
            Some(Value::Int(1024))
        ),
        Some(Answer::Yes)
    );
    assert_eq!(
        answer_value!(
            "too-large-for-stack",
            Some(Value::Bool(true)),
            Some(Value::Int(200))
        ),
        Some(Answer::No)
    );
}

#[test]
fn only_an_empty_array_counts_as_the_empty_list_setting() {
    assert_eq!(
        answer_value!(
            "allowed-prefixes",
            None::<Value>,
            Some(Value::Array(Vec::new()))
        ),
        Some(Answer::Yes)
    );
    assert_eq!(
        answer_value!(
            "allowed-prefixes",
            None::<Value>,
            Some(Value::Array(vec![Value::Str(Text::of("to"))]))
        ),
        Some(Answer::No)
    );
}

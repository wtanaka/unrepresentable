//! `clippy.toml` knobs that have a stricter setting than their default, with
//! the strict value this repository chose, for the pinned clippy version.
//!
//! Booleans and enum strings sit at their strict setting. A list whose default
//! is non-empty must be `[]`, so an absent `ignore-interior-mutability` fails; an
//! absent knob passes only when its default equals its strict setting. Each
//! integer knob has a [`Scale`], which `check --against` uses to tell loosening from
//! tightening.
//!
//! Chosen values for the integer and list knobs, with the default and the reason:
//!
//! | Knob | Default | Chosen | Why |
//! | --- | --- | --- | --- |
//! | `too-many-lines-threshold` | 100 | 60 | One screenful. |
//! | `cognitive-complexity-threshold` | 25 | 15 | Functions that need more are split. |
//! | `excessive-nesting-threshold` | 0 (off) | 5 | 4 needed about 35 refactors in this code base; 5 needed 4. |
//! | `too-many-arguments-threshold` | 7 | 5 | Past five, a parameter struct. |
//! | `type-complexity-threshold` | 250 | 150 | Complex types get a named alias. |
//! | `large-error-threshold` | 128 | 64 | Small error types. |
//! | `enum-variant-size-threshold` | 200 | 64 | Small variants. |
//! | `enum-variant-name-threshold` | 3 | 2 | Variant names must not repeat the enum name. |
//! | `struct-field-name-threshold` | 3 | 2 | Field names must not repeat the struct name. |
//! | `future-size-threshold` | 16384 | 1024 | Small futures. |
//! | `array-size-threshold` | 16384 | 512 | Large arrays go on the heap. |
//! | `pass-by-value-size-limit` | 256 | 64 | Larger values are passed by reference. |
//! | `vec-box-size-threshold` | 4096 | 16384 | Higher is stricter: `Vec<Box<T>>` is flagged for `T` up to this size. |
//! | `max-trait-bounds` | 3 | 2 | Few bounds per generic. |
//! | `max-include-file-size` | 1000000 | 10000 | Small `include_str!` inputs. |
//! | `stack-size-threshold` | 512000 | 16384 | Small stack frames. |
//! | `too-large-for-stack` | 200 | 1024 | Higher is stricter: boxed locals and `vec!` up to this size are flagged. |
//! | `single-char-binding-names-threshold` | 4 | 0 | No single-character bindings (`min_ident_chars` is denied too). |
//! | `unnecessary-box-size` | 128 | 512 | Higher is stricter: returning `Box<T>` is flagged for `T` below this size. |
//! | `check-grouped-late-init` | false | true | Check grouped late initialisation. |
//! | `allowed-idents-below-min-chars` | `i j x y z w n` | `[]` | No single-letter names. |
//! | `allowed-prefixes` | `to as into from ..` | `[]` | No exempt name prefixes. |
//!
//! `excessive-nesting-threshold` uses `Scale::LowerZeroOff`, because `0` turns
//! the check off. `too-large-for-stack`, `unnecessary-box-size` and
//! `vec-box-size-threshold` use `Scale::Higher`: they are upper bounds on the
//! size of what is linted, so a larger number makes the lints fire more.
//! These three have no natural maximum: their chosen values are caps picked
//! well above the default, not true limits, and a stricter value later means
//! editing this table.
//!
//! Knobs left at their default, and why: `check-private-items` (true raises
//! 195 `missing_panics_doc` hits, all on `#[test]` fns, whose job is to assert);
//! `min-ident-chars-threshold` (2 flags 183 two-letter names such as `ws` and
//! `ch`); `absolute-paths-max-segments` (1 flags 116 paths);
//! `literal-representation-threshold` (this table itself needs round decimal
//! values); `max-suggested-slice-pattern-length`, `verbose-bit-mask-threshold`
//! and `const-literal-digits-threshold` (no clear direction of strictness).
//! `missing-docs-in-crate-items` and `missing-docs-allow-unused` already default
//! to the strict value.
//!
//! Only keys the pinned clippy accepts belong here. The clippy documentation
//! lists `min-ident-chars-lint-trait-impl`, which clippy 0.1.98 rejects as an
//! unknown key, so it is absent. The accepted keys are printed by the
//! `unknown field` error of a `clippy.toml` that holds a bogus key.
//!
//! A new clippy version needs a human decision: it has new lints and knobs, so
//! moving the toolchain also means changing [`CLIPPY_VERSION`], this table and
//! possibly the ledger. Review the table against the configuration documentation
//! of each new clippy; editing `rust-toolchain.toml` alone exits 2.
use crate::toml::Value;

use crate::types::{Answer, Holds as _, Name};

/// The clippy version this table was written for.
pub(crate) const CLIPPY_VERSION: &str = "0.1.98";

/// Every `clippy.toml` knob with a stricter setting than its default.
pub(crate) const KNOBS: &[Knob] = &[
    Knob {
        default: Setting::Bool(true),
        name: KnobName("accept-comment-above-attributes"),
        scale: Scale::Exact,
        strictest: Setting::Bool(false),
    },
    Knob {
        default: Setting::Bool(true),
        name: KnobName("accept-comment-above-statement"),
        scale: Scale::Exact,
        strictest: Setting::Bool(false),
    },
    Knob {
        default: Setting::Bool(true),
        name: KnobName("allow-comparison-to-zero"),
        scale: Scale::Exact,
        strictest: Setting::Bool(false),
    },
    Knob {
        default: Setting::Bool(true),
        name: KnobName("allow-exact-repetitions"),
        scale: Scale::Exact,
        strictest: Setting::Bool(false),
    },
    Knob {
        default: Setting::Bool(true),
        name: KnobName("allow-expect-in-consts"),
        scale: Scale::Exact,
        strictest: Setting::Bool(false),
    },
    Knob {
        default: Setting::Bool(true),
        name: KnobName("allow-large-stack-frames-in-tests"),
        scale: Scale::Exact,
        strictest: Setting::Bool(false),
    },
    Knob {
        default: Setting::Bool(true),
        name: KnobName("allow-mixed-uninlined-format-args"),
        scale: Scale::Exact,
        strictest: Setting::Bool(false),
    },
    Knob {
        default: Setting::Bool(true),
        name: KnobName("allow-unwrap-in-consts"),
        scale: Scale::Exact,
        strictest: Setting::Bool(false),
    },
    Knob {
        default: Setting::Listed,
        name: KnobName("allowed-idents-below-min-chars"),
        scale: Scale::Exact,
        strictest: Setting::Empty,
    },
    Knob {
        default: Setting::Listed,
        name: KnobName("allowed-prefixes"),
        scale: Scale::Exact,
        strictest: Setting::Empty,
    },
    Knob {
        default: Setting::Int(0x4000),
        name: KnobName("array-size-threshold"),
        scale: Scale::Lower,
        strictest: Setting::Int(512),
    },
    Knob {
        default: Setting::Bool(true),
        name: KnobName("avoid-breaking-exported-api"),
        scale: Scale::Exact,
        strictest: Setting::Bool(false),
    },
    Knob {
        default: Setting::Bool(false),
        name: KnobName("check-grouped-late-init"),
        scale: Scale::Exact,
        strictest: Setting::Bool(true),
    },
    Knob {
        default: Setting::Bool(false),
        name: KnobName("check-incompatible-msrv-in-tests"),
        scale: Scale::Exact,
        strictest: Setting::Bool(true),
    },
    Knob {
        default: Setting::Bool(false),
        name: KnobName("check-inconsistent-struct-field-initializers"),
        scale: Scale::Exact,
        strictest: Setting::Bool(true),
    },
    Knob {
        default: Setting::Int(25),
        name: KnobName("cognitive-complexity-threshold"),
        scale: Scale::Lower,
        strictest: Setting::Int(15),
    },
    Knob {
        default: Setting::Bool(false),
        name: KnobName("enforce-iter-loop-reborrow"),
        scale: Scale::Exact,
        strictest: Setting::Bool(true),
    },
    Knob {
        default: Setting::Int(3),
        name: KnobName("enum-variant-name-threshold"),
        scale: Scale::Lower,
        strictest: Setting::Int(2),
    },
    Knob {
        default: Setting::Int(200),
        name: KnobName("enum-variant-size-threshold"),
        scale: Scale::Lower,
        strictest: Setting::Int(64),
    },
    Knob {
        default: Setting::Int(0),
        name: KnobName("excessive-nesting-threshold"),
        scale: Scale::LowerZeroOff,
        strictest: Setting::Int(5),
    },
    Knob {
        default: Setting::Int(0x4000),
        name: KnobName("future-size-threshold"),
        scale: Scale::Lower,
        strictest: Setting::Int(1024),
    },
    Knob {
        default: Setting::Listed,
        name: KnobName("ignore-interior-mutability"),
        scale: Scale::Exact,
        strictest: Setting::Empty,
    },
    Knob {
        default: Setting::Int(128),
        name: KnobName("large-error-threshold"),
        scale: Scale::Lower,
        strictest: Setting::Int(64),
    },
    Knob {
        default: Setting::Bool(false),
        name: KnobName("lint-commented-code"),
        scale: Scale::Exact,
        strictest: Setting::Bool(true),
    },
    Knob {
        default: Setting::Str("WellKnownTypes"),
        name: KnobName("matches-for-let-else"),
        scale: Scale::Exact,
        strictest: Setting::Str("AllTypes"),
    },
    Knob {
        default: Setting::Int(3),
        name: KnobName("max-fn-params-bools"),
        scale: Scale::Exact,
        strictest: Setting::Int(0),
    },
    Knob {
        default: Setting::Int(1_000_000),
        name: KnobName("max-include-file-size"),
        scale: Scale::Lower,
        strictest: Setting::Int(10000),
    },
    Knob {
        default: Setting::Int(3),
        name: KnobName("max-struct-bools"),
        scale: Scale::Exact,
        strictest: Setting::Int(0),
    },
    Knob {
        default: Setting::Int(3),
        name: KnobName("max-trait-bounds"),
        scale: Scale::Lower,
        strictest: Setting::Int(2),
    },
    Knob {
        default: Setting::Int(256),
        name: KnobName("pass-by-value-size-limit"),
        scale: Scale::Lower,
        strictest: Setting::Int(64),
    },
    Knob {
        default: Setting::Str("PubliclyExported"),
        name: KnobName("pub-underscore-fields-behavior"),
        scale: Scale::Exact,
        strictest: Setting::Str("AllPubFields"),
    },
    Knob {
        default: Setting::Int(4),
        name: KnobName("single-char-binding-names-threshold"),
        scale: Scale::Lower,
        strictest: Setting::Int(0),
    },
    Knob {
        default: Setting::Int(512_000),
        name: KnobName("stack-size-threshold"),
        scale: Scale::Lower,
        strictest: Setting::Int(0x4000),
    },
    Knob {
        default: Setting::Int(3),
        name: KnobName("struct-field-name-threshold"),
        scale: Scale::Lower,
        strictest: Setting::Int(2),
    },
    Knob {
        default: Setting::Int(200),
        name: KnobName("too-large-for-stack"),
        scale: Scale::Higher,
        strictest: Setting::Int(1024),
    },
    Knob {
        default: Setting::Int(7),
        name: KnobName("too-many-arguments-threshold"),
        scale: Scale::Lower,
        strictest: Setting::Int(5),
    },
    Knob {
        default: Setting::Int(100),
        name: KnobName("too-many-lines-threshold"),
        scale: Scale::Lower,
        strictest: Setting::Int(60),
    },
    Knob {
        default: Setting::Int(250),
        name: KnobName("type-complexity-threshold"),
        scale: Scale::Lower,
        strictest: Setting::Int(150),
    },
    Knob {
        default: Setting::Int(128),
        name: KnobName("unnecessary-box-size"),
        scale: Scale::Higher,
        strictest: Setting::Int(512),
    },
    Knob {
        default: Setting::Bool(false),
        name: KnobName("upper-case-acronyms-aggressive"),
        scale: Scale::Exact,
        strictest: Setting::Bool(true),
    },
    Knob {
        default: Setting::Int(4096),
        name: KnobName("vec-box-size-threshold"),
        scale: Scale::Higher,
        strictest: Setting::Int(0x4000),
    },
    Knob {
        default: Setting::Bool(false),
        name: KnobName("warn-on-all-wildcard-imports"),
        scale: Scale::Exact,
        strictest: Setting::Bool(true),
    },
    Knob {
        default: Setting::Bool(false),
        name: KnobName("warn-unsafe-macro-metavars-in-private-macros"),
        scale: Scale::Exact,
        strictest: Setting::Bool(true),
    },
];

/// A knob value, as far as this table distinguishes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Setting {
    /// A boolean setting.
    Bool(bool),
    /// An array of strings that must be empty.
    Empty,
    /// An integer setting.
    Int(i64),
    /// A non-empty list: the default of knobs whose strict setting is `[]`.
    Listed,
    /// A fixed string setting.
    Str(&'static str),
}

/// How an integer knob orders its values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scale {
    /// Not an integer knob: only the strict setting itself is acceptable.
    Exact,
    /// A higher number is stricter.
    Higher,
    /// A lower number is stricter.
    Lower,
    /// A lower number is stricter, and `0` switches the check off.
    LowerZeroOff,
}

/// A knob name as a static table entry.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KnobName(&'static str);

impl KnobName {
    /// The knob's name as a string.
    pub(crate) const fn text(self) -> &'static str {
        self.0
    }
}

/// A `clippy.toml` knob and the settings relevant to it.
pub(crate) struct Knob {
    /// The value clippy uses when the knob is unset.
    pub default: Setting,
    /// The knob's key in `clippy.toml`.
    pub name: KnobName,
    /// How integer values of this knob are ordered by strictness.
    pub scale: Scale,
    /// The strictest value, which this repository requires.
    pub strictest: Setting,
}

/// The old and new value of one knob.
pub(crate) struct Transition<'val> {
    /// The value after the change, or `None` if the knob is absent.
    pub new: Option<&'val Value>,
    /// The value before the change, or `None` if the knob was absent.
    pub old: Option<&'val Value>,
}

/// An integer setting, compared by value.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Whole(i64);

impl Whole {
    /// Position on the strictness scale: a lower rank is stricter.
    const fn rank(self, scale: Scale) -> Self {
        match scale {
            Scale::LowerZeroOff if self.0 == 0 => Self(i64::MAX),
            Scale::Exact | Scale::Higher | Scale::Lower | Scale::LowerZeroOff => self,
        }
    }
}

/// Looks up the knob called `name`.
#[must_use]
pub(crate) fn find(name: &Name) -> Option<&'static Knob> {
    KNOBS.iter().find(|knob| knob.name.text() == name.as_str())
}

/// Converts a TOML value into a known `Setting`, if it is one.
#[must_use]
fn to_setting(value: &Value) -> Option<Setting> {
    match value {
        Value::Bool(flag) => Some(Setting::Bool(*flag)),
        Value::Int(number) => Some(Setting::Int(*number)),
        Value::Str(text) => KNOBS
            .iter()
            .flat_map(|knob| [knob.default, knob.strictest])
            .find_map(|setting| match setting {
                Setting::Str(known) if known == text.as_str() => Some(Setting::Str(known)),
                Setting::Str(_)
                | Setting::Bool(_)
                | Setting::Int(_)
                | Setting::Empty
                | Setting::Listed => None,
            }),
        Value::Array(items) if items.is_empty() => Some(Setting::Empty),
        Value::Raw(_) | Value::Array(_) | Value::Table(_) => None,
    }
}

/// Does `value` equal the strictest setting of `knob`?
pub(crate) fn is_strictest(knob: &Knob, value: &Value) -> Answer {
    (to_setting(value) == Some(knob.strictest)).answer()
}

/// The setting in force for `value`, falling back to the default when the knob is unset.
#[must_use]
fn effective(knob: &Knob, value: Option<&Value>) -> Option<Setting> {
    match value {
        Some(found) => to_setting(found),
        None => Some(knob.default),
    }
}

/// Is the new value at least as strict as the old one? Unknown settings are not.
pub(crate) fn at_least_as_strict(knob: &Knob, change: &Transition<'_>) -> Answer {
    let (Some(before), Some(after)) = (effective(knob, change.old), effective(knob, change.new))
    else {
        return Answer::No;
    };
    if before == after {
        return Answer::Yes;
    }
    match knob.scale {
        Scale::Exact => (after == knob.strictest).answer(),
        Scale::Higher => match (whole_of(before), whole_of(after)) {
            (Some(old_value), Some(new_value)) => (new_value >= old_value).answer(),
            (Some(_) | None, None) | (None, Some(_)) => (after == knob.strictest).answer(),
        },
        Scale::Lower | Scale::LowerZeroOff => match (whole_of(before), whole_of(after)) {
            (Some(old_value), Some(new_value)) => {
                (new_value.rank(knob.scale) <= old_value.rank(knob.scale)).answer()
            }
            (Some(_) | None, None) | (None, Some(_)) => (after == knob.strictest).answer(),
        },
    }
}

/// The integer inside `setting`, if it is an integer.
#[must_use]
const fn whole_of(setting: Setting) -> Option<Whole> {
    match setting {
        Setting::Int(value) => Some(Whole(value)),
        Setting::Bool(_) | Setting::Str(_) | Setting::Empty | Setting::Listed => None,
    }
}

#[cfg(test)]
mod tests;

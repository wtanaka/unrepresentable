//! `clippy.toml` knobs that have a stricter setting than their default, with
//! the strict value this repository chose, for each clippy version in
//! [`CLIPPY_VERSIONS`]. Every listed version was reviewed against this table.
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
//! moving the toolchain also means reviewing this table and adding the version
//! to [`CLIPPY_VERSIONS`], and possibly the ledger. Review the table against the configuration documentation
//! of each new clippy; editing `rust-toolchain.toml` alone exits 2.

#[cfg(test)]
mod tests;

/// The definitions of this module.
mod logic;

pub(crate) use logic::{
    at_least_as_strict, find, is_strictest, Transition, CLIPPY_VERSIONS, KNOBS,
};

#[cfg(test)]
pub(crate) use logic::{Knob, Scale, Setting};

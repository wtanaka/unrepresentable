//! Rule ids. Every rule is on, in every scope, for every crate.
/// Flags calls that abort on failure, such as `unwrap` and `expect`.
pub(crate) const ABORT_UNWRAP: RuleId = RuleId("abort-unwrap");

/// Flags panicking macros such as `panic!`, `todo!` and `unreachable!`.
pub(crate) const ABORT_PANIC: RuleId = RuleId("abort-panic");

/// Flags `assert!`-family macros used in non-test code.
pub(crate) const ABORT_ASSERT: RuleId = RuleId("abort-assert");

/// Flags methods that silently replace an error or absence with a default value.
/// This rule, `abort-unwrap` and the `wrapping_*` checks are name based: they run
/// on method calls and on path calls (`Option::unwrap(x)`) and do not look at the
/// receiver type.
pub(crate) const SILENT_FALLBACK: RuleId = RuleId("silent-fallback");

/// Flags values that are computed and then thrown away, such as `let _ = ..`.
pub(crate) const DISCARDED_VALUE: RuleId = RuleId("discarded-value");

/// Flags functions named `*_raw` or `*_unchecked`. It applies to every function,
/// not only public ones; the id is a misnomer.
pub(crate) const PUB_FN_NAME: RuleId = RuleId("pub-fn-name");

/// Flags named struct fields declared with bare primitive types (`bool` and
/// `char` count as primitives). Tuple-struct fields are never checked: that is
/// where a newtype stores its primitive.
pub(crate) const PRIMITIVE_FIELD: RuleId = RuleId("primitive-field");

/// Flags function signatures that use bare primitive types: integers, floats and
/// strings in parameters and returns, `char` in either position and `bool` in
/// returns (a `bool` parameter is `bool-param`). A newtype's own accessor and
/// constructor methods are exempt through the `newtypes` table; impls of std
/// traits such as `PartialEq::eq` are not.
pub(crate) const PRIMITIVE_SIGNATURE: RuleId = RuleId("primitive-signature");

/// Flags function parameters of type `bool`.
pub(crate) const BOOL_PARAM: RuleId = RuleId("bool-param");

/// Flags functions with several parameters of the same type that could be swapped.
pub(crate) const TRANSPOSABLE_PARAMS: RuleId = RuleId("transposable-params");

/// Flags functions returning `Option` that are not marked `#[must_use]`.
pub(crate) const OPTION_NOT_MUST_USE: RuleId = RuleId("option-not-must-use");

/// Flags wrapping or saturating arithmetic and `Wrapping`/`Saturating` types.
pub(crate) const MODULAR_ARITH: RuleId = RuleId("modular-arith");

/// Flags use of raw arithmetic operators.
pub(crate) const ARITH_OPERATOR: RuleId = RuleId("arith-operator");

/// Flags lint suppression attributes such as `#[allow]`. This tool's own rules
/// have no in-source suppression; only a ledger-backed `#[expect]` exists, and
/// only for compiler and clippy lints.
pub(crate) const SUPPRESSION: RuleId = RuleId("suppression");

/// Flags hash-based collections whose iteration order is unspecified: std,
/// `hashbrown`, `rustc_hash` and `ahash` (`indexmap` is ordered).
pub(crate) const UNORDERED_COLLECTION: RuleId = RuleId("unordered-collection");

/// Flags paths that leak or hand-manage memory: `Box::leak`, `Vec::leak`,
/// `String::leak`, `OsString::leak`, `PathBuf::leak` (all stable; `Rc` and `Arc`
/// have no `leak`), `std::mem::ManuallyDrop` in any use
/// (type, `new`, `drop`, `take`) and a method call named `leak` with no
/// arguments. Leaked memory is never reclaimed and hides who owns the value. The
/// rule is name and path based: it resolves paths like `unordered-collection`
/// does but does not know receiver types. `mem::forget` is not here (the
/// `clippy::mem_forget` lint is required by `lint-coverage`), and `into_raw` is
/// not here because a raw pointer can only be used through `unsafe`, which
/// `manifest-lints` forbids.
pub(crate) const MEMORY_LEAK: RuleId = RuleId("memory-leak");

/// Flags every path with an `f32` or `f64` segment (types, constants, associated
/// functions, `use` leaves) and float literals.
pub(crate) const FLOAT_TYPE: RuleId = RuleId("float-type");

/// Flags access to ambient state such as the environment, clock, filesystem,
/// `std::os` or network, including file-system probes through `Path`.
pub(crate) const AMBIENT_INPUT: RuleId = RuleId("ambient-input");

/// Flags `as` casts between numeric types.
pub(crate) const NUMERIC_CAST: RuleId = RuleId("numeric-cast");

/// Flags wildcard or catch-all `match` arms. `refutable-shortcut` and this rule
/// know only enums declared in the workspace (resolved through the index);
/// externally defined enums are invisible to them.
/// A `..` that this rule reports is not reported again as `struct-rest-pattern`.
pub(crate) const CATCH_ALL_ARM: RuleId = RuleId("catch-all-arm");

/// Flags shortcuts that handle only one case of a refutable pattern, such as `if let`.
pub(crate) const REFUTABLE_SHORTCUT: RuleId = RuleId("refutable-shortcut");

/// Flags a `..` rest pattern in a struct, tuple-struct or enum-variant pattern
/// whose type is declared in the workspace, so a field added later is not
/// silently ignored by every reader. A type with no fields is
/// not flagged. The fix is to name every field, binding an ignored one to an
/// underscore-prefixed name. Slice and tuple patterns are not struct
/// patterns. A `..` that `catch-all-arm` already reports in a `match` is left to
/// that rule and is not reported twice.
pub(crate) const STRUCT_REST_PATTERN: RuleId = RuleId("struct-rest-pattern");

/// Flags `#[derive(Default)]` on a workspace struct or union (also inside
/// `cfg_attr`), and struct-update syntax whose base is a `default()` call
/// (`Account { balance, ..Default::default() }`) on a workspace struct. A derived
/// `Default` fills every field with a blank value that may be invalid for the
/// domain, and the update form hides which fields were chosen. A hand-written
/// `impl Default` is a deliberate constructor and is not checked. An enum may
/// derive `Default`: Rust requires an explicit `#[default]` variant, so a
/// variant was chosen on purpose. Calls such as `T::default()` are not flagged.
pub(crate) const DEFAULT_CONSTRUCTION: RuleId = RuleId("default-construction");

/// Flags a `let` that binds an `_`-prefixed name (not bare `_`, which
/// `discarded-value` owns) that is never read later in the enclosing block. Any
/// later occurrence of the name in the rest of the block counts as a read,
/// including in closures, nested blocks, macro tokens and inline format
/// arguments, so a shadowing `let` is never reported. The rule is syntax based:
/// fn and closure parameters and `match`, `if let` and `for` patterns are not
/// covered. A name such as `_guard` is not exempt: an exemption would hide
/// an error.
pub(crate) const UNUSED_UNDERSCORE_BINDING: RuleId = RuleId("unused-underscore-binding");

/// Flags `drop(..)`, `std::mem::drop(..)` and `core::mem::drop(..)` (resolved
/// through `use` and aliases) whose argument is directly a call, method call,
/// macro call or `?` expression: the freshly produced value is discarded.
/// `drop(guard)` of a named binding stays legal. The rule is syntax based.
pub(crate) const DROPPED_CALL_RESULT: RuleId = RuleId("dropped-call-result");

/// Flags an `Err(_)`, `Err(..)` or `Err(_name)` `match` arm with an empty body,
/// and `if let Ok(..) = ..` or `while let Ok(..) = ..` with no `else`: each drops
/// an error silently. `Err` and `Ok` that resolve to a workspace item are skipped.
/// A `_` that `catch-all-arm` reports, and a pattern that `refutable-shortcut`
/// reports (it names a workspace variant), are not reported again. `Option` and
/// `None` are out of scope. The rule is syntax based.
pub(crate) const SWALLOWED_ERROR: RuleId = RuleId("swallowed-error");

/// Flags `cfg(clippy)` conditional compilation.
pub(crate) const CFG_CLIPPY: RuleId = RuleId("cfg-clippy");

/// Flags use of items from a configured ban set; the finding rule is `banned:<id>`.
pub(crate) const BANNED: RuleId = RuleId("banned");

/// Flags tests that use plain assertion macros.
pub(crate) const TEST_ASSERTION: RuleId = RuleId("test-assertion");

/// Flags a `return` in a test fn's own body that comes before the first
/// assertion macro in source order: the test can then pass without asserting.
/// It counts every spelling (`return;`, `return Ok(())`, a `let .. else` whose
/// else block returns) except `return Err(..)`, which fails the
/// test, and uses the macros of `TEST_ASSERTION_MACROS`. There is no switch and
/// no configuration key.
pub(crate) const TEST_EARLY_RETURN: RuleId = RuleId("test-early-return");

/// Flags macro definitions that lack an explanatory comment.
pub(crate) const MACRO_NEEDS_COMMENT: RuleId = RuleId("macro-needs-comment");

/// Flags test attributes that are not allowed by the policy.
pub(crate) const TEST_ATTRIBUTES: RuleId = RuleId("test-attributes");

/// Flags a `Cargo.toml` whose lint tables do not match the required configuration.
pub(crate) const MANIFEST_LINTS: RuleId = RuleId("manifest-lints");

/// Flags lint configuration files or settings found outside the sanctioned places.
pub(crate) const STRAY_LINT_CONFIG: RuleId = RuleId("stray-lint-config");

/// Flags compiler or clippy lints that the lint configuration leaves uncovered.
pub(crate) const LINT_COVERAGE: RuleId = RuleId("lint-coverage");

/// Flags dependencies that the policy forbids.
pub(crate) const FORBIDDEN_DEPENDENCY: RuleId = RuleId("forbidden-dependency");

/// Flags source references to forbidden crates or paths.
pub(crate) const FORBIDDEN_REFERENCE: RuleId = RuleId("forbidden-reference");

/// Flags source files that the module walk never reached, and symbolic links
/// that point outside the workspace or at nothing. It scans hidden
/// directories as well and skips only nested workspace members, `target`, `.git`,
/// `.jj` and canary directories, so a `.rs` file in a `tests/` helper directory
/// that no test target reaches is a finding.
pub(crate) const UNREACHED_FILE: RuleId = RuleId("unreached-file");

/// Flags `include!`-style macros whose target cannot be scanned.
pub(crate) const UNSCANNABLE_INCLUDE: RuleId = RuleId("unscannable-include");

/// Flags macro invocations whose body could not be parsed for scanning, and
/// `macro_rules!` definitions that have a finding or a transcriber that does not
/// parse.
pub(crate) const UNPARSED_MACRO: RuleId = RuleId("unparsed-macro");

/// Flags a lint preference that the configuration omits.
pub(crate) const PREFERENCE_OMISSION: RuleId = RuleId("preference-omission");

/// Flags a finding count that grew beyond the recorded baseline.
pub(crate) const BASELINE: RuleId = RuleId("baseline");

/// Flags a rule that has no canary file proving it fires.
pub(crate) const CANARY: RuleId = RuleId("canary");

/// Flags one change that touches both judge paths and code paths.
///
/// Reported at `.unrepresentable/config:1:1`, since no better file exists to
/// attach it to, and not baselinable. It is evaluated over the whole changed-path
/// set, including untracked files, so on a fresh repository where everything is
/// "added" it fires at once. Judge and code paths are not exclusive: a path
/// matching both sets both flags. Protected-path changes count as judge changes.
pub(crate) const PAIRING: RuleId = RuleId("judge-code-pairing");

/// Rules a canary must cover (`banned:<id>` entries are added per ban set).
pub(crate) const CANARY_RULES: &[RuleId] = &[
    ABORT_UNWRAP,
    ABORT_PANIC,
    ABORT_ASSERT,
    SILENT_FALLBACK,
    DISCARDED_VALUE,
    PUB_FN_NAME,
    PRIMITIVE_FIELD,
    PRIMITIVE_SIGNATURE,
    BOOL_PARAM,
    TRANSPOSABLE_PARAMS,
    OPTION_NOT_MUST_USE,
    MODULAR_ARITH,
    ARITH_OPERATOR,
    SUPPRESSION,
    UNORDERED_COLLECTION,
    MEMORY_LEAK,
    FLOAT_TYPE,
    AMBIENT_INPUT,
    NUMERIC_CAST,
    CATCH_ALL_ARM,
    REFUTABLE_SHORTCUT,
    STRUCT_REST_PATTERN,
    DEFAULT_CONSTRUCTION,
    UNUSED_UNDERSCORE_BINDING,
    DROPPED_CALL_RESULT,
    SWALLOWED_ERROR,
    CFG_CLIPPY,
    TEST_ASSERTION,
    TEST_EARLY_RETURN,
    MACRO_NEEDS_COMMENT,
    TEST_ATTRIBUTES,
    UNSCANNABLE_INCLUDE,
    UNPARSED_MACRO,
];

/// Method names that abort on failure when called on `Option` or `Result`.
pub(crate) const UNWRAP_FAMILY: &[&str] = &[
    "unwrap",
    "expect",
    "unwrap_err",
    "expect_err",
    "unwrap_unchecked",
];

/// Method names that silently turn an error or absence into a fallback value.
pub(crate) const SILENT_FALLBACK_NAMES: &[&str] = &[
    "unwrap_or",
    "unwrap_or_else",
    "unwrap_or_default",
    "map_or",
    "map_or_else",
    "ok",
    "err",
    "or",
    "or_else",
    "get_or_insert",
    "get_or_insert_with",
];

/// Names of macros that unconditionally panic.
pub(crate) const PANIC_MACROS: &[&str] = &["panic", "unreachable", "todo", "unimplemented"];

/// Names of assertion macros that abort when a condition fails.
pub(crate) const ASSERT_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
];

/// Names of assertion macros that tests must not use directly.
pub(crate) const TEST_ASSERTION_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "assert_matches",
    "prop_assert",
    "prop_assert_eq",
    "prop_assert_ne",
];

/// Names of macros that print to stdout or stderr.
pub(crate) const PRINT_MACROS: &[&str] = &["print", "println", "eprint", "eprintln", "dbg"];

/// Paths of functions that terminate or abort the process.
pub(crate) const PANIC_PATHS: PatternTable = PatternTable(&[
    "std::process::exit",
    "std::process::abort",
    "std::hint::unreachable_unchecked",
]);

/// Paths of std modules and types that read ambient state.
pub(crate) const AMBIENT_PATHS: PatternTable = PatternTable(&[
    "std::env",
    "std::os",
    "std::path::Path::canonicalize",
    "std::path::Path::exists",
    "std::path::Path::is_dir",
    "std::path::Path::is_file",
    "std::path::Path::is_symlink",
    "std::path::Path::metadata",
    "std::path::Path::read_dir",
    "std::path::Path::read_link",
    "std::path::Path::symlink_metadata",
    "std::path::Path::try_exists",
    "std::path::PathBuf::canonicalize",
    "std::path::PathBuf::exists",
    "std::path::PathBuf::is_dir",
    "std::path::PathBuf::is_file",
    "std::path::PathBuf::is_symlink",
    "std::path::PathBuf::metadata",
    "std::path::PathBuf::read_dir",
    "std::path::PathBuf::read_link",
    "std::path::PathBuf::symlink_metadata",
    "std::path::PathBuf::try_exists",
    "std::time::SystemTime",
    "std::time::Instant",
    "std::fs",
    "std::process",
    "std::net",
    "std::thread",
    "std::io::stdin",
    "std::io::stdout",
    "std::io::stderr",
]);

/// Paths of std collections and hashers with unspecified iteration order.
pub(crate) const UNORDERED_PATHS: PatternTable = PatternTable(&[
    "std::collections::HashMap",
    "std::collections::HashSet",
    "std::collections::hash_map",
    "std::collections::hash_set",
    "std::hash::RandomState",
    "std::collections::hash_map::RandomState",
    "std::hash::DefaultHasher",
    "hashbrown::HashMap",
    "hashbrown::HashSet",
    "hashbrown::HashTable",
    "hashbrown::hash_map",
    "hashbrown::hash_set",
    "hashbrown::hash_table",
    "rustc_hash::FxHashMap",
    "rustc_hash::FxHashSet",
    "rustc_hash::FxBuildHasher",
    "ahash::AHashMap",
    "ahash::AHashSet",
    "ahash::RandomState",
]);

/// Paths of std items that leak memory or take it out of its owner's hands. The
/// bare `Box`, `Vec` and `String` entries are the prelude spellings.
pub(crate) const LEAK_PATHS: PatternTable = PatternTable(&[
    "Box::leak",
    "String::leak",
    "Vec::leak",
    "std::boxed::Box::leak",
    "std::ffi::OsString::leak",
    "std::mem::ManuallyDrop",
    "std::path::PathBuf::leak",
    "std::string::String::leak",
    "std::vec::Vec::leak",
]);

/// Paths of std wrapper types with modular arithmetic semantics.
pub(crate) const MODULAR_TYPE_PATHS: PatternTable =
    PatternTable(&["std::num::Wrapping", "std::num::Saturating"]);

/// Method names that convert to or from floating point values.
pub(crate) const FLOAT_METHODS: &[&str] = &["from_f32", "from_f64", "as_f64", "to_f64"];

/// Names of all numeric primitive types, integer and float.
pub(crate) const NUMERIC_PRIMITIVES: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32",
    "f64",
];

/// Names of the integer primitive types.
pub(crate) const INTEGER_PRIMITIVES: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
];

/// Attribute paths recognised as marking a function as a test.
pub(crate) const BUILTIN_TEST_ATTRIBUTES: &[&str] = &[
    "test",
    "tokio::test",
    "rstest",
    "rstest::rstest",
    "test_case",
    "test_case::test_case",
    "async_std::test",
    "actix_rt::test",
];

/// A static table of path patterns such as `std::fs`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PatternTable(&'static [&'static str]);

impl PatternTable {
    /// The path patterns in this table.
    pub(crate) const fn entries(self) -> &'static [&'static str] {
        self.0
    }
}

/// The stable id of a rule, as findings print it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RuleId(&'static str);

impl RuleId {
    /// The rule id string, such as `abort-unwrap`.
    pub(crate) const fn name(self) -> &'static str {
        self.0
    }
}

# Known limits

This page lists code and situations that `unrepresentable` does not analyze or
does not report. The guarantees in the [README](../README.md) do not apply to
anything listed here. The rules are described in [rules.md](rules.md).

## Macro bodies

The tool does not expand macros. Most rules run on the body of a macro
invocation or definition as written.

- An invocation (`vec![..]`, `assert!(..)`, `weird!(..)`) has its body parsed as
  items, then statements, then comma-separated expressions. The first parse that
  works gets every rule. When none works, a token-level scan runs and the
  invocation is reported as `unparsed-macro`, which can be baselined. The token
  scan recognizes only: `abort-unwrap`, `silent-fallback` and `modular-arith`
  method names; macro and path names (`abort-panic`, `ambient-input`, `banned`,
  `forbidden-reference`, `memory-leak`, `unordered-collection`); `let _ =`
  statements and `_ =` assignments; `as` casts; floats; and `#[allow]`-style
  attributes. The scan reads no patterns, items, statements or calls, so
  `struct-rest-pattern`, `default-construction`, `unused-underscore-binding`,
  `dropped-call-result` and `swallowed-error` do not run on unparsed macro
  bodies: a `..`, a `#[derive(Default)]`, a `let _x`, a `drop(call())` or an
  empty `Err` arm in one is not reported (the invocation is still reported as
  `unparsed-macro`).
- A macro named in `[[macro-comment]]` (for example `proptest!`) is not reported
  as `unparsed-macro`; the required comment above the invocation is the record.
- A `macro_rules!` definition is scanned rule by rule: each transcriber is
  parsed with `$x:ident`, `$t:ty` and the other metavariables replaced by
  placeholders and `$( .. )*` replaced by one copy of the repeated text, and
  then gets every rule. The expansions are never scanned. A definition that has
  a finding, or a transcriber that does not parse even then, is also reported as
  `unparsed-macro`.
- Findings inside macro bodies carry the macro as an item (`macro name!`).
- The token-level scan reports `test-early-return` for a bare `return` in a fn
  that follows a test attribute, except `return Err(..)`. The scan does not
  determine where a closure ends, so it ignores a `return` after a `|` that
  starts a closure, or after `async`, up to the next `;` or `,` at the same
  level. The scan does not recognise `let .. else`.

An RAII guard (a lock, a transaction, a timer) must live to the end of a scope,
and `let _ = ..` and an unused `_`-prefixed binding are rejected
(`discarded-value`, `unused-underscore-binding`). Two patterns remain: a
closure-scoped helper that takes the resource and a closure
(`with_lock(&m, |state| ..)`), or an explicitly named binding that is `drop`ped
at the end. A helper can constrain its argument with a sealed marker trait that
only the approved guard types implement, so that a `Result` cannot be passed in.
The tool ships no library code for this; the project that needs the helper
writes it.

## What the tool does not cover

- A deleted workflow or a CI check that is not marked as required. The tool
  reports changes to protected paths (see [change control](change-control.md));
  the repository settings must make the check required.
- Types. The tool parses syntax and resolves names. Method rules match by name
  (a user method called `ok` is flagged; another name for the same call is not),
  `catch-all-arm` applies only to workspace enums, `discarded-value` does not
  report ignored `#[must_use]` returns, and `numeric-cast` detects the `as`
  expression but not the operand's type.
- Generated code: external proc-macro output and `OUT_DIR` files (reported as
  `unscannable-include`). Macro expansions are not scanned; see
  [Macro bodies](#macro-bodies) for what runs on bodies and `macro_rules!`
  templates. `#[cfg]` is not evaluated; every branch is scanned.
- Symbolic links: links to files and directories are followed for targets and
  for `unreached-file` (a link that points outside the workspace is a finding
  and is not loaded). Discovery of workspace members from `members` patterns
  does not follow directory links.
- Workspace members: as in cargo, a path dependency inside the workspace
  directory that is not in `exclude` is a member and needs a `[member.<name>]`
  role in `.unrepresentable/config` like any other member.
- Cross-crate resolution: a workspace library is known by its `[lib] name` and
  by every dependency rename (`foo = { package = "core-api", .. }`) found in any
  manifest of the workspace. A rename is not limited to the manifest that
  declares the rename.
- `test-early-return` looks only at where a `return` sits in source order
  relative to the first assertion macro. `continue` can still skip every case in
  a loop, so a test whose assertions are all behind a `continue` passes with
  nothing checked. `assert!(true)` counts as an assertion. An assertion inside a
  helper fn does not count (the same as `test-assertion`), so a test that
  asserts only through a helper and returns early is still reported. A `return`
  after the first assertion is not checked, so assertions placed after a
  conditional `return` that is itself after the first assertion may never run.
  Every `return` counts except `return Err(..)`, which is exempt because that
  return fails the test; the exemption applies to a call of any path whose last
  segment is `Err`.
- `struct-rest-pattern` covers workspace types only: a struct, tuple struct or
  enum variant that the index resolves (through `use`, aliases, re-exports and
  `Self`), or that is declared in a function body. A type from a dependency or
  std, a name that does not resolve and a name that a glob import of another
  crate may provide are not flagged, and neither are slice or tuple patterns. A
  type alias declared in a function body counts as a workspace type, because the
  index records items declared in function bodies by name only. `catch-all-arm`
  owns a `..` it reports (a `..` beside workspace variants in a `match`); every
  other `..` is `struct-rest-pattern`. The field count comes from the index, so
  a struct or variant declared in a function body (count unknown) is flagged
  even when that struct or variant has no fields.
- `default-construction` does not check that a hand-written `impl Default` has
  valid values: the rule only removes the blank derive and the hidden-field
  update. A derive generated by an external proc-macro, or written inside a
  macro body that only the token-level scan can read, is not analyzed (the token
  scan reads no items). A struct declared in a function body is resolved as
  other rules resolve such a struct: its derive is reported, and a struct update
  on it counts as a workspace struct. `T::default()` calls are never flagged, by
  design: with the derive banned, a workspace type's `Default` is a deliberate
  constructor, and a call on an external type is outside the rule.
- `unused-underscore-binding`, `dropped-call-result` and `swallowed-error` are
  syntax and name based. These three rules do not use type information, so they
  do not detect the following: `let r = fallible(); drop(r);` (a named binding
  that is dropped), a helper such as `fn ignore<T>(_v: T) {}` called as
  `ignore(fallible())`, `.is_ok()` and `.is_err()` peeks, unused fn and closure
  parameters, and `Err(e) => {}` with a name that does not start with `_`.
  Types, `#[must_use]` and a human review of `src/**` are the defence against
  these; the tool does not cover them. `swallowed-error` matches `Err(..)` and
  `Ok(..)` by name, and only in `match` arms, `if let` and `while let`.
- `memory-leak` is name and path based. The rule matches the listed paths
  however they are imported, plus a zero-argument method call named `leak`. The
  rule does not know receiver types: a user method `x.leak()` is flagged, and a
  leak through a qualified path such as `<Box<T>>::leak(b)` is not. `into_raw`
  and `mem::forget` are not covered (see [the rule table](rules.md)). Only
  `leak` functions that exist in stable std are listed (checked against the std
  documentation); an unstable function is added when it is stabilised, and a
  zero-argument `.leak()` call is flagged either way.
- Enums declared inside a function body are known to `catch-all-arm` and
  `refutable-shortcut` only for the code of that block.
- A baselined finding that moves to another item in the same (rule, crate)
  baseline group. The comparison counts baseline entries per group, so the move
  is not reported as new debt.
- Whether the approving human read the change.
- Dependencies. A dependency is code that runs in CI. The set is small and
  pinned (see [Dependencies](maintainers/development.md#dependencies)), and a
  compromised release of one of them would run with the tool's access.
  `cargo-deny` and the exact pins reduce this risk and do not remove it.
- TOML floats and dates are read as text, not as numbers, because the tool never
  computes with them. An integer outside 64 bits is read as text too; it is an
  error only where the tool needs an integer (a config count), not for the file.
  A manifest whose meaning depends on the value of such a number is not
  interpreted.
- `.unrepresentable/config` must be valid TOML. Earlier versions accepted
  `baseline = "..."` together with `[[baseline]]` tables, which TOML forbids.
  The baseline tables are now `[[baseline-entry]]`; a config that still uses
  `[[baseline]]` exits 2 with `duplicate key` and a file:line:col position until
  the headers are renamed.
- A `default-range` that selects no changes exits 2. A run on a branch equal to
  its base (a push to `main`) therefore needs `check --against` or
  `check --range`. `default-range` is for local use only, because the change
  under review chooses the base it selects (see
  [`default-range`](configuration.md#default-range-is-not-a-trust-anchor)); CI
  passes `--range` or `--against` itself.
- `stray-lint-config` also reads `.cargo/config` and `.cargo/config.toml` in
  directories above the workspace root, because `rustflags` there change what
  clippy reports. A `.cargo/config` with `rustflags` in a home directory above
  the workspace is therefore a finding on that machine, so results depend on the
  machine.
- A change to the tool that weakens a rule only on inputs that are not in the
  fixtures, the canaries, the public corpus or the hidden corpus (see
  [tests/README.md](../tests/README.md)).
- Nested `include!` is not followed (`unscannable-include`).
- Build scripts that modify the source tree, the contents of dependencies, and
  CI configuration outside the protected paths.

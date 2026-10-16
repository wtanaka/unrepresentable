# Tests

`unrepresentable` is tested with three bodies of cases. This page lists them and
says which cases cover which rule. Rule ids are defined in
[docs/rules.md](../docs/rules.md).

1. In-repository cases, run by `cargo test` (through `ci/cargo.sh`) and the
   tool's own modes:
   - `tests/fixtures/` (sources, `.in` suffix) with expected findings in
     `tests/expected/`, run by `tests/suite/`.
   - The built-in `self-test` canaries (`src/selftest.rs`).
   - The rule canaries in `tests/canaries/` (marked `//~ rule-id`, run by
     `canaries`).
   - The clippy canaries in `canaries/rust/src/bin/` (one per lint). Fixtures
     and canaries are append-only judge paths: adding one is free; modifying or
     deleting one needs human approval.
2. The public corpus: crates downloaded and pinned by sha256 in a lock file in
   this repository. The CI process that reviews a pull request runs the base and
   the head binary over them and fails on any finding the head loses
   (`LOST_FINDING`).
3. A hidden corpus, kept outside the repository, that the CI process that
   reviews a pull request also runs the base and head binaries over.

A change to the tool that weakens a rule only on inputs outside all three bodies
is not detected (see [docs/limits.md](../docs/limits.md)). The "head binary" is
the tool as built from the pull request, compared by the CI process that reviews
a pull request with the "base binary" built from the default branch.

## What covers which rule

- File-level rules have a canary with a `//~ rule-id` mark on the offending
  line. `canaries` fails on any diagnostic that is not marked with its own id on
  its own line, so a canary cannot contain an unmarked second violation.
- Rules that need a whole repository cannot be expressed by a mark, because they
  read manifests, the lock file, git history or ledgers: `manifest-lints`,
  `stray-lint-config`, `lint-coverage`, `forbidden-dependency`,
  `forbidden-reference`, `unreached-file`, `judge-code-pairing`, `baseline`,
  `canary` and `preference-omission`. These rules are covered by `tests/suite/`
  (fixtures and sandbox repositories).
- `tests/suite/meta.rs` reads every `RuleId("..")` of `src/rules.rs` and
  requires a row with a named positive test and a named negative test for each.
  A rule without those rows fails the suite.

## Not published

The crate published to crates.io leaves `tests/` out (the explicit `include` in
the root `Cargo.toml`). The helpers in `tests/support/mod.rs` (`lint_tables`,
`overlay`) read the `[workspace.lints*]` tables of this repository's root
manifest, and `cargo package` rewrites those into per-crate `[lints]` tables, so
the tests fail when run from the unpacked crate (this was checked). Run them on
a checkout of the repository.

## Running

CI runs the tests with `sh ci/checks.sh test` (see
[ci/cargo.sh](../ci/cargo.sh)). Tests that run the cargo calls made by the tool
put a `cargo` stand-in on `$CARGO` that logs its arguments
(`Sandbox::install_cargo_logger`, `run_with_cargo` in `tests/suite/cli.rs`). The
tests assert that `--release --locked` follow the subcommand and that the
sandbox has no `target/debug`.

## Canaries in this repository

Every rule has a canary: known-bad code that must produce a finding at a marked
line. A rule that stops reporting violations without any error would let
violations pass unnoticed. How an adopter uses `self-test` and `canaries` is
described in [docs/lint-coverage.md](../docs/lint-coverage.md).

- The rule canaries are in `tests/canaries/`.
- The clippy canaries are in `canaries/rust/src/bin/`, one file per lint.
- A file in the clippy canary directory that is neither a canary directly in it
  nor loaded by one through `mod` is a `canary` finding. Canary directories are
  not scanned as code, so no other check would read such a file.
- Rules that need a whole repository (manifests, lint coverage, dependencies,
  ledgers, baseline) have no file canaries. The integration tests in
  `tests/suite/` cover them (see "What covers which rule" above).

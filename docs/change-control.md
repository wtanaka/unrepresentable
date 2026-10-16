# Ratchet and change control

This page describes how `unrepresentable` limits what one change can do to the
checks themselves. With `check --against` or `check --range`, each changed file
is put in one of two kinds:

- A _judge path_ is a file that configures or runs the checks: CI workflows,
  toolchain and lint settings, and `.unrepresentable/`. The word comes from the
  `judge` key in the config and the `judge-code-pairing` rule.
- A _code path_ is a source file matched by a glob in the `code` key of
  `.unrepresentable/config`.

A change that edits both kinds is a finding, so that a change cannot loosen a
check and use the loosened check in the same step. A change that loosens a check
or adds an exception exits 3, which means a human must approve it. The
_baseline_ is the list of existing findings that are accepted
(`[[baseline-entry]]` in the config); `check --ratchet` keeps it from growing.

- `check --ratchet` rewrites the `[[baseline-entry]]` entries: it keeps entries
  that still match a finding, drops stale entries, and re-keys entries of
  renamed items only where the number of entries per (rule, crate) does not
  rise. `--ratchet` never raises that number.
- `check --against` / `check --range` classify every changed path. A change that
  tightens a check needs no approval; a change that loosens a check or adds an
  exception exits 3. Judge files and code files must not change in the same
  change (`judge-code-pairing`). The built-in judge paths (`ALWAYS_JUDGE` in
  `src/against.rs`) are, at the repository root only, `.unrepresentable/**`,
  `rust-toolchain`, `rust-toolchain.toml`, `Cargo.lock`, `.github/**` and
  `ci/unrepresentable.sh`; and, at the root or in any directory, `clippy.toml`,
  `.clippy.toml`, `deny.toml`, `.deny.toml`, `Cargo.toml`, `.cargo/**`,
  `.gitignore`, `build.rs`, `rustfmt.toml` and `.rustfmt.toml`; plus the `judge`
  globs. Protected paths (`PROTECTED` in the same file) always need approval,
  even when added: `ci/unrepresentable.sh`,
  `.github/workflows/unrepresentable.yml`, `.github/workflows/policy.yml` and a
  `CODEOWNERS` file in any directory (`**/CODEOWNERS`, which covers the root,
  `.github/` and `docs/`). The two workflow names are defaults:
  `unrepresentable.yml` is the consumer-side name of the workflow that runs the
  tool and `policy.yml` is this repository's merge policy. A project that names
  its CI workflow otherwise still has that file under `.github/**` as a judge
  path; list other CI files under `judge`.
- A path is a judge or code path if the config at the base of the range or the
  config of the change says so, so a change that removes a `code` glob while
  editing code still pairs. A new judge file is free (append-only) only when the
  config at the base lists that file; an `append-only` glob that the change
  itself adds exempts nothing. Adding or removing a `canary-dirs` entry needs
  approval.

Details: `src/baseline.rs`, `src/against.rs`, and `.unrepresentable/config` for
the `judge`, `code` and `append-only` keys.

## Boundaries

`.unrepresentable/config` can declare `[[forbidden-dependency]]` (crate A must
not reach crate B through manifests and `Cargo.lock`), `[[forbidden-reference]]`
(files in a glob must not name a path) and `role = "pure"` members, which must
not use anything listed under `ambient-input` in [rules.md](rules.md).

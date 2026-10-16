# Configuration

This page describes `.unrepresentable/config`, the configuration file of
`unrepresentable`, and the other files `init` writes.

## What `init` writes

`init` creates the directory `.unrepresentable/` at the repository root and
writes three files in that directory:

- `.unrepresentable/config`, with `code` set to the `src` and `tests`
  directories of every member, every member `pure`, `baseline = "shrink-only"`,
  and a baseline of everything currently found (the existing findings that are
  accepted and may only shrink);
- `.unrepresentable/ledger.md`;
- `.unrepresentable/canary-exemptions.md`.

The location is fixed; no flag or key moves it. `init` exits with an error if
the config exists. After `init`, review the config. Edit `code` to the paths
that are code (a config without a non-empty `code` exits 2, because
`judge-code-pairing` has no code paths to pair with otherwise). Mark crates that
do I/O as `shell` with their `ambient` entries. Commit the config, then delete
any existing grep-based checks for the same patterns. Set
`baseline = "forbidden"` once the baseline is empty.

The baseline holds only findings in source files. Repository-level findings
(`manifest-lints`, `lint-coverage`, `stray-lint-config` and the like) cannot be
baselined. A project that has not copied this repository's `[workspace.lints]`
tables, `clippy.toml` and `rust-toolchain.toml` gets these findings on the first
`check` (exit 1; for a bare crate, one `manifest-lints` finding and a
`lint-coverage` finding per lint that is not denied). Fix them first.

## `default-range` is not a trust anchor

`default-range` is a convenience for local use, not a trust anchor. It is read
from the config under review, so a change can rewrite it, and that value chooses
the base. Example: a commit that loosens the config and sets
`default-range = "HEAD~1"`, followed by a trivial commit, is compared only with
its parent, which already holds the loosened config. The tool reduces this risk
(the range used is the `default-range` of the config at the selected base when
that config has one, and a config range that selects no changes exits 2), but
cannot remove it, because every commit the tool could compare against was
written by the change. Only a base that does not come from the change can be
trusted. CI must therefore always pass `--range` or `--against` with a base that
CI chose itself, as `ci/checks.sh owntree`, the workflows and
`ci/unrepresentable.sh` do, and must not rely on `default-range`.

## Configuration notes

- A `shell` member lists in `ambient` entries what the member may touch, for
  example `[member.io] ambient = [{ path = "std::fs", files = ["src/io.rs"] }]`.
  `files` is a list of globs; empty means every file of the member. Naming a
  prefix of an allowed path (`std::io` on the way to `std::io::stdout`) is not a
  violation by itself.
- Print macros are checked as the paths `std::print`, `std::println`,
  `std::eprint`, `std::eprintln` and `std::dbg`; `env!` and `option_env!` as
  `std::env`. A shell crate that prints must list those, or a prefix such as
  `std`.
- `role = "pure"` ignores `ambient` entries.
- Adding or changing an `ambient` entry is an exception under `check --against`
  (`EXCEPTION_ADDED`, exit 3).
- Every member needs a `[member.<name>]` role, or the run exits 2. `init` marks
  every member `pure`; `init` cannot know which crates are shells.
- `config::TOP_KEYS` is the full list of keys. No key disables a rule (see
  [Turning rules off](../README.md#turning-rules-off)).
- `code` is required and must list at least one glob. `judge` and `append-only`
  add to the built-in judge paths (see
  [Ratchet and change control](change-control.md)).
- Globs (`judge`, `code`, `append-only`, `arith-files`, `files`) are
  repo-relative: `*` stays within one segment, `**` as a whole segment spans
  directories, `?` is one character, `[` is the literal character. A glob that
  is empty or starts with `/` exits 2 (such a glob could never match). A
  trailing `/` means the directory and everything below it: `a/` matches `a` and
  `a/b/c`, not `ab/c`. Workspace `members` patterns in `Cargo.toml` are read as
  cargo reads them, with `[ab]` character classes; a pattern that the `glob`
  crate rejects exits 2.
- Two `[[ban]]` sets may not share an id.

## Fingerprints and the baseline contract

Baseline entries and `#[expect]` ledger rows identify an item by a path of
identifiers joined with `/`, so the path format is part of the contract:

- `mod a`, then nested modules, then the item;
- `impl Foo`, `impl Display for Kind`, `impl From<A> for Y` (trait path and type
  name carry their generic arguments by identifier, so `impl From<A> for Y` and
  `impl From<B> for Y` are different items);
- `fn name`, `struct S`, `union U`, `enum E / variant V`, `trait T`, `const C`,
  `static S`, `type T`, `macro name!` (an invocation or a `macro_rules` named
  `name`) and `struct S / field f`.

Example: `mod a / impl Display for Kind / fn fmt`. The path is built from
identifiers and not from source text, so reformatting does not change the path.
Renaming any component changes the fingerprint; `check --ratchet` can re-key
baseline entries for renamed items (see [change control](change-control.md)).

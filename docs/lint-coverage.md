# Lint coverage and canaries

This page describes the `lint-coverage` rule, the omission ledger
(`.unrepresentable/ledger.md`), and the `self-test` and `canaries` subcommands,
which check that the rules still report violations.

## `lint-coverage` and the omission ledger

Every `check` enforces `lint-coverage`. A project must deny every rustc lint and
every clippy lint, or give each omitted lint a row in the omission ledger with a
written reason. A lint that is neither denied nor in the ledger is a
`lint-coverage` finding.

- Clippy must be the exact version the strictest-knob table in `src/knobs.rs`
  was written for; otherwise the run exits 2. A new clippy version needs a human
  decision about the new lints and knobs, so a toolchain bump is a code change
  in `unrepresentable`, not only an edit of `rust-toolchain.toml`.
- The `restriction` group cannot be set wholesale. Each restriction lint is
  named individually in `Cargo.toml`.

Omission-ledger rows (`.unrepresentable/ledger.md`, Reason column) are of three
kinds:

- Contradiction pairs: the `Contradicts` column names a denied lint that the
  omitted one conflicts with (for example `implicit_return` and
  `needless_return`).
- Preference omissions (`Contradicts` is `-`): printed as a
  `preference-omission` note on every run. The note does not change the exit
  code.
- Unstable lints: the marker `unstable` in `Contradicts`. The marker is checked:
  `rustc` must still report the lint as unstable, so a lint that becomes stable
  turns the row of that lint into a `lint-coverage` finding.

## `self-test` and `canaries`

A rule that stops reporting violations without any error would let violations
pass unnoticed. Two subcommands check that each rule still reports:

- `self-test` runs canaries compiled into the binary and needs no repository.
  Run `self-test` before trusting a binary.
- `canaries` runs the canaries of the repository being checked. It reads the
  directories in `canary-dirs` in `.unrepresentable/config`. A rule canary is a
  file that begins with `// canary role=pure` or `// canary role=shell`, and
  each line that must produce a finding carries a mark `//~ <rule-id>`. A canary
  with no marks is a `canary` finding. If `clippy-canaries` is set in the
  config, `canaries` also runs one clippy canary binary per lint.

Neither subcommand takes a change range; each exits 2 when given `--against` or
`--range` (see [Command line](cli.md#subcommands)).

How this repository tests its own canaries is described in
[tests/README.md](../tests/README.md#canaries-in-this-repository).

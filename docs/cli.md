# Command line

This page is the command-line reference for `unrepresentable`.

```
unrepresentable check [--against REF | --range RANGE] [--ratchet] <root>
unrepresentable init [--against REF | --range RANGE] <root>
unrepresentable self-test [<root>]
unrepresentable canaries <root>
```

## Subcommands

A subcommand is required. A bare `unrepresentable <root>` and any other usage
error print the error to standard error and exit 2. `--help` and `--version`
print to standard output and exit 0. `--against` and `--range` exclude each
other, and only the subcommands that list them accept them.

- `check` runs all checks over a change range, applies the baseline and prints
  the report (see [The change range](#the-change-range)). `--ratchet` first
  rewrites the baseline to drop stale entries and re-key moved ones.
- `init` writes `.unrepresentable/config` with a baseline of the current
  findings, and stub ledgers. `init` refuses to overwrite a config. With
  `--against` or `--range` it also checks that no config exists at the base.
- `self-test` runs the canaries built into the binary. `<root>` is optional and
  defaults to `.`. `self-test` needs no git or cargo.
- `canaries` checks the rule canaries of the project and, when configured, its
  clippy canaries.

## The change range

`check` checks the files that changed between a base commit and the current
commit. The range of commits is chosen by the first of these that is present:

1. `--range RANGE`, a git range such as `origin/main...HEAD`.
2. `--against REF`, which means `REF...HEAD`.
3. `default-range` in `.unrepresentable/config`, for local use only.

When none is present, the run exits 2. In `A...B`, the base is the merge base of
`A` and `B`; in `A..B`, the base is `A`. CI must pass `--range` or `--against`
and must not rely on `default-range` (see
[Configuration](configuration.md#default-range-is-not-a-trust-anchor)).

`check` also needs:

- the full git history, because a shallow clone is refused;
- the pinned toolchain and clippy installed (see
  [Running in CI](ci.md#toolchain-prerequisites)).

## Output

`range: ...`, then `path:line:col: rule: message` per finding, then notes, then
`approval: REASON` lines.

## Exit codes

| Exit | Meaning                                                                                                                                                                                                                                                                                                                                 |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0    | Clean                                                                                                                                                                                                                                                                                                                                   |
| 1    | At least one finding (takes precedence over exit 3)                                                                                                                                                                                                                                                                                     |
| 2    | The tool could not check the code: usage, unparsable Rust/config/ledger, git or I/O failure, unknown clippy version, missing config or ledger, no range, a config range that selects no changes, a config that cannot work (no `code`, a bad glob, a `[[forbidden-dependency]]` naming an unknown crate in `crate` or `must-not-reach`, an invalid `must-never-reach` name, or listing neither `must-not-reach` nor `must-never-reach`) |
| 3    | No findings, but the change touches something a human must approve; each reason is printed as `approval: ...`                                                                                                                                                                                                                           |

In CI, exit 1 and exit 2 must fail the check. Exit 3 means the code rules pass
and a human must approve the change, because the change touches the checks
themselves (config, ledgers, baseline, protected paths) and not only ordinary
code. Approval reasons are listed in `src/against.rs`.

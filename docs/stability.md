# Stability

`unrepresentable` is in alpha.

## What alpha means

- There is no backwards compatibility promise between any two releases,
  including two alpha releases with the same `0.0.1` core.
- A release can change or remove anything listed under "What is likely to
  change" without a deprecation period.
- Alpha releases are marked as pre-releases on GitHub. Cargo does not choose a
  pre-release unless the version is written out in full.

## What is likely to change

- Rules: ids, what the rules reject, and new always-on rules. A rule that is new
  in a release applies to every project that upgrades, so code that passed
  before can fail.
- The config format (`.unrepresentable/config`), the ledger format, the canary
  exemptions format and the baseline and fingerprint format. An old file may be
  refused, or its entries may stop matching.
- Exit behaviour: which situations exit 1, 2 or 3, and the text of reports.
- The command line options and the modes.
- The release files: names, and the installer script `unrepresentable.sh`.

## Upgrade advice

- Pin an exact version:
  `cargo install unrepresentable --locked --version =<exact version>`, or the
  installer script of one release, which pins the version and the sha256 of each
  binary.
- Upgrade in a separate pull request, run the tool, and fix or record the new
  findings there. Do not let CI use a newer version than the one pinned.
- Run `self-test` after installing.

#!/bin/sh
# Prints the `version` of the `[package]` table of a Cargo.toml read from
# standard input.
#
# usage: sh ci/version.sh < Cargo.toml
#
# Only a line `version = "X.Y.Z"` or `version = "X.Y.Z-PRERELEASE"` (an optional
# trailing comment is allowed) inside `[package]` counts. A `version` key of a
# dependency table, of `[workspace.package]` or of another table is not taken,
# and `version.workspace = true` is not a version. The script prints nothing and
# exits 1 when there is no version, and exits 1 when it is not `MAJOR.MINOR.PATCH`
# with an optional SemVer pre-release suffix: `-` and dot-separated identifiers
# of `[0-9A-Za-z-]`, none empty, numeric ones without leading zeros. Build
# metadata (`+...`) and a `v` prefix are refused.
#
# Users: `ci/checks.sh package`.
set -eu

version=$(awk '
  /^[ \t]*\[/ { in_package = ($0 ~ /^[ \t]*\[package\][ \t]*(#.*)?$/) }
  in_package && /^[ \t]*version[ \t]*=[ \t]*"[^"]*"[ \t]*(#.*)?$/ {
    value = $0
    sub(/^[^"]*"/, "", value)
    sub(/".*$/, "", value)
    print value
    exit
  }
')
printf '%s\n' "$version" | grep -Eq '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-(0|[1-9][0-9]*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*)(\.(0|[1-9][0-9]*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*))*)?$' || exit 1
printf '%s\n' "$version"

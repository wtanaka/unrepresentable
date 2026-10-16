#!/bin/sh
# Run head code (a build, the tests, the mutation tests) as an unprivileged user
# with no network, in the sandbox directory staged by policy/stage-sandbox.sh.
# usage: SANDBOX_DIR=DIR offline.sh <command> [args...]
#
# This is sandbox.sh with SANDBOX_DIR required (so it also runs in its own PID
# namespace and every process of the user is ended before this returns). The
# command runs as uid 65534,
# not as the runner user: it has no `sudo`, cannot write the base checkout, the
# runner's temporary directory, the files the job hands to later steps or the
# staged toolchain and tools, and has no access to a token. What it may write is DIR/home,
# DIR/cargo, DIR/mut, the copy of the head under DIR/work and /tmp.
set -eu

[ -n "${SANDBOX_DIR:-}" ] || {
  printf 'offline.sh: SANDBOX_DIR is not set\n' >&2
  exit 2
}
here=$(CDPATH='' cd "$(dirname "$0")" && pwd)
exec sh "$here/sandbox.sh" "$@"

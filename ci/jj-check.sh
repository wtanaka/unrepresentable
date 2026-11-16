#!/bin/sh
# Runs `unrepresentable check` from a jj workspace.
#
# usage: sh ci/jj-check.sh [CHECK_ARGS...]
#
# Purpose: a secondary jj workspace has no `.git`, so git cannot see the
# repository there, and the git HEAD of the shared store belongs to another
# workspace. This script points git at the git store behind the workspace,
# gives git a private index seeded with the parent of the working-copy commit,
# and runs `check --head COMMIT` with that parent as COMMIT. Pass the same
# arguments as for `check`, for example `--against main .`; the root argument
# is required, as for `check`. Run it from the root of the workspace.
#
# Requires `jj` and `git` on PATH. The tool is `$UNREPRESENTABLE` when that is
# set, else it is built and run through ci/cargo.sh.
#
# Related facts:
# - The working-copy commit must have exactly one parent. The edits of the
#   working copy are compared with that parent, as `git diff HEAD` does in a
#   git checkout.
# - The private index lives in a temporary directory that is removed on exit,
#   so the index of the shared store is never touched.
# - A change that exists only in jj is visible to git once jj has exported it,
#   which every jj command does in a colocated repository and in a workspace
#   of a repository with a git backend.
set -eu

here=$(CDPATH='' cd "$(dirname "$0")" && pwd)

die() {
  printf 'ci/jj-check.sh: %s\n' "$1" >&2
  exit 2
}

case "${1:-}" in
  -h | --help)
    printf '%s\n' \
      'usage: sh ci/jj-check.sh [CHECK_ARGS...]' \
      'Runs `check --head PARENT` with git pointed at the store behind the jj' \
      'workspace in the current directory. The header comment has the details.'
    exit 0
    ;;
esac

[ -d .jj ] || die "run it from the root of a jj workspace (no .jj here)"
command -v jj >/dev/null 2>&1 || die "jj is not on PATH"
command -v git >/dev/null 2>&1 || die "git is not on PATH"

# In a secondary workspace `.jj/repo` is a file naming the shared repository
# relative to `.jj`; in the first workspace it is the repository itself.
if [ -f .jj/repo ]; then
  repo_ref=$(cat .jj/repo)
  repo=$(CDPATH='' cd .jj && CDPATH='' cd "$repo_ref" && pwd -P)
else
  repo=$(CDPATH='' cd .jj/repo && pwd -P)
fi
store="$repo/store"
[ -f "$store/git_target" ] || die "the jj repository has no git backend"
target=$(cat "$store/git_target")
git_dir=$(CDPATH='' cd "$store" && CDPATH='' cd "$target" && pwd -P)

parents=$(jj log -r '@-' --no-graph -T 'commit_id ++ "\n"')
count=$(printf '%s\n' "$parents" | grep -c .)
[ "$count" = 1 ] || die "the working-copy commit must have exactly one parent"
parent=$(printf '%s\n' "$parents" | head -n 1)

scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

GIT_DIR="$git_dir"
GIT_WORK_TREE=$(pwd -P)
GIT_INDEX_FILE="$scratch/index"
export GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE
git read-tree "$parent"

if [ -n "${UNREPRESENTABLE:-}" ]; then
  "$UNREPRESENTABLE" check --head "$parent" "$@"
else
  sh "$here/cargo.sh" run --quiet -- check --head "$parent" "$@"
fi

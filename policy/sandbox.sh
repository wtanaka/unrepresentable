#!/bin/sh
# Run a command with no network, no privileges and an empty environment.
# usage: [SANDBOX_DIR=DIR] [SANDBOX_UID=N] [SANDBOX_SCRATCH=DIR] sandbox.sh [NAME=VALUE...] <command> [args...]
#
# The command runs as uid/gid N (65534, `nobody`, unless SANDBOX_UID says
# 65533) with no supplementary groups. It runs in a network namespace with a
# loopback interface (brought up here, so tests that bind 127.0.0.1 work) and
# no other interface, in a PID namespace of its own (no other process is
# visible in it, and `--kill-child` and the kernel end everything in it when its first
# process ends), and with no_new_privs, so neither `sudo` nor a setuid program
# can give the privileges back. Nothing of the caller's environment is passed
# on: the command gets PATH, HOME and CARGO_TERM_COLOR, and the NAME=VALUE
# words in front of it.
#
# When the command is done, and before this script returns (so before the caller
# reads anything the command wrote), every process of that uid is killed
# (`sudo -n pkill -KILL -u UID`) and `pgrep -u UID` must find none, or this
# script exits 2. A process that outlived its command could rewrite the files
# the caller is about to read. So no two sandboxed commands may run at the same
# time as the same uid; the jobs run them one after the other.
#
# Without SANDBOX_DIR, PATH is /usr/bin:/bin and HOME is /nonexistent. This is
# how the differential check runs the two binaries on the inputs.
#
# With SANDBOX_DIR (a directory made by policy/stage-sandbox.sh), PATH starts
# with the staged toolchain and tools, HOME is DIR/home, CARGO_HOME is DIR/cargo
# and CARGO_NET_OFFLINE is true. The toolchain is called directly (not through
# rustup), so a rust-toolchain file of the code being run is not read.
#
# SANDBOX_UID is 65534 or 65533. The differential check runs the base binary as
# 65534 and the head binary as 65533, so neither can read, execute, signal or
# write what the other owns (see policy/stage-sandbox.sh --private-bin).
#
# SANDBOX_SCRATCH is a directory made by the caller that the command may write:
# an absolute path of A-Za-z0-9/._-, owned by the caller, not a link, holding
# nothing but the empty directories cargo-home, home and tmp. It is given to
# the uid (mode 0700, so no other user can enter it) before the command starts.
# It can be given once: the second time it is no longer the caller's.
#
# Everything the command should read must be readable by the uid and nothing it
# may not change may be writable by it. The caller must be allowed to `sudo -n`
# (the GitHub-hosted Linux runners are).
set -eu

die() {
  printf 'sandbox.sh: %s\n' "$1" >&2
  exit 2
}

[ "$#" -ge 1 ] || die "usage: sandbox.sh [NAME=VALUE...] <command> [args...]"

uid="${SANDBOX_UID:-65534}"
case "$uid" in
  65534 | 65533) ;;
  *) die "SANDBOX_UID must be 65534 or 65533" ;;
esac

path='/usr/bin:/bin'
home='/nonexistent'
dir="${SANDBOX_DIR:-}"
if [ -n "$dir" ]; then
  case "$dir" in
    /*) ;;
    *) die "SANDBOX_DIR must be an absolute path" ;;
  esac
  case "$dir" in
    *[!A-Za-z0-9/._-]*) die "SANDBOX_DIR has a character outside A-Za-z0-9/._-" ;;
  esac
  [ -d "$dir/toolchain/bin" ] || die "$dir is not a staged sandbox directory"
  path="$dir/toolchain/bin:$dir/bin:/usr/bin:/bin"
  home="$dir/home"
fi

scratch="${SANDBOX_SCRATCH:-}"
if [ -n "$scratch" ]; then
  case "$scratch" in
    /*) ;;
    *) die "SANDBOX_SCRATCH must be an absolute path" ;;
  esac
  case "$scratch" in
    *[!A-Za-z0-9/._-]*) die "SANDBOX_SCRATCH has a character outside A-Za-z0-9/._-" ;;
  esac
  { [ -d "$scratch" ] && [ ! -L "$scratch" ]; } || die "$scratch is not a directory"
  [ -n "$(find "$scratch" -maxdepth 0 -user "$(whoami)")" ] ||
    die "$scratch is not the caller's (it is made by the caller and given once, for one run)"
  for part in cargo-home home tmp; do
    { [ -d "$scratch/$part" ] && [ ! -L "$scratch/$part" ]; } || die "$scratch has no $part directory"
  done
  other="$(find "$scratch" -mindepth 1 ! -path "$scratch/cargo-home" ! -path "$scratch/home" ! -path "$scratch/tmp" | head -n 1)"
  [ -z "$other" ] || die "$scratch is not fresh: it holds $other"
  sudo -n /usr/bin/chown -R "$uid:$uid" "$scratch"
  sudo -n /usr/bin/chmod 0700 "$scratch"
fi

set -- PATH="$path" HOME="$home" CARGO_TERM_COLOR=never "$@"
if [ -n "$dir" ]; then
  set -- CARGO_HOME="$dir/cargo" CARGO_NET_OFFLINE=true "$@"
fi

# `unshare --fork` starts the shell below as the first process (PID 1) of the
# new PID namespace. The shell brings the loopback interface up while it still
# has privileges, then runs the command. Because the command is the last one of
# the `sh -c` string, dash replaces the shell with it (exec), so setpriv, and
# then the command as the unprivileged uid, takes over PID 1; with a shell that
# does not do that, the shell stays PID 1 and the command is its child. The
# kill guarantee holds in both cases: when PID 1 of a PID namespace ends, the
# kernel kills every other process in the namespace, and `unshare` returns only
# after PID 1 has ended (`--kill-child` also kills it if `unshare` itself is
# killed). The pkill and pgrep below still check the uid afterwards.
status=0
sudo -n /usr/bin/unshare --net --pid --fork --kill-child --mount-proc -- \
  /bin/sh -c '/usr/sbin/ip link set lo up || exit 125; "$@"' sandbox \
  /usr/bin/setpriv --reuid="$uid" --regid="$uid" --clear-groups --no-new-privs \
  /usr/bin/env -i "$@" || status=$?

killed=0
sudo -n /usr/bin/pkill -KILL -u "$uid" || killed=$?
[ "$killed" -le 1 ] || die "pkill failed (exit $killed)"
tries=0
while pgrep -u "$uid" >/dev/null 2>&1; do
  tries=$((tries + 1))
  [ "$tries" -le 50 ] || die "processes of uid $uid are still running after the command ended"
  sleep 0.1
  killed=0
  sudo -n /usr/bin/pkill -KILL -u "$uid" || killed=$?
  [ "$killed" -le 1 ] || die "pkill failed (exit $killed)"
done
exit "$status"

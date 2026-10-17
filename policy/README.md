# Merge policy

This page describes the merge policy of this repository: the tooling under
`policy/` and `.github/` that decides whether a pull request merges. The goals
are that a pull request is merged automatically when CI passes, that no pull
request can weaken a check and still merge automatically, and that every change
that could weaken a check goes to a human who is shown what they approve.

Trust model: comments, pull request text and check names do not affect the
decision. The merge decision is recomputed by code from the default branch for
one exact head commit, from git and the GitHub API. The only trusted signal is a
check run named `policy`, posted for that commit by the GitHub App
`unrepresentable-policy`. The ruleset requires `policy` with that App as the
expected source. Other jobs are inputs to the decision through `needs:` and
same-run artifacts; the decision does not look up their check runs by name.

## Files

| Path                                                                                          | Role                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.github/workflows/policy.yml`                                                                | The one workflow for pull requests and the default branch. `pull_request_target`: build-base (builds the policy tool and the base binary once), classify, self-lint, head-checks, build-head, differential, differential-hidden, mutants, then `policy-evaluate` (App token, check run, comment, then a second token only for the merge). `push` and `workflow_dispatch`: skip-check, then fmt, clippy-main, clippy-policy, deny, test, policytest, canaries, self-checks and audit-workflows in parallel. Every job is gated by `github.event_name`.                                                                                                                                                                                                                                                                                                                                 |
| `.github/workflows/review-signal.yml`                                                         | `pull_request_review`: does nothing; only starts the next workflow.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `.github/workflows/policy-review.yml`                                                         | `workflow_run`: a `build` job (no environment, no secret) compiles the policy tool; the `review` job (environment `policy`) runs it after checking its sha256, and re-evaluates approval for the exact current head; updates the `policy` check run.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `.github/workflows/policy-sweep.yml`                                                          | Hourly re-evaluation of all open PRs (same `build` then `sweep` split) and the advisories check.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `.github/workflows/settings-audit.yml`                                                        | Daily comparison of live settings and recent merge commits with `settings.json`; opens or updates an issue on drift. The policy tool is built in a job with no secret and handed over by sha256.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `.github/workflows/release.yml`                                                               | Tag-triggered release (reproducible Linux builds, SHA256SUMS, `unrepresentable.sh`, trusted publishing).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `.github/CODEOWNERS`                                                                          | `* @wtanaka`, `/policy/ @wtanaka`, `/.github/ @wtanaka`. Only individual users are resolved by the policy; a team owner approves nothing.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `policy/Cargo.toml`, `policy/src/`                                                            | The `unrepresentable-policy` binary (workspace member, four exact-pinned third-party dependencies (listed under "Policy crate notes"), held to the tool's own rules). One subcommand per job step: `classify`, `post decide`, `post merge`, `review`, `push-skip`, `release-gate`, `audit`, `differential`, `self-lint`, `head-jobs`, `fetch-head`, `install-tool`, `lock-check`, `make-installer`. Workflows build it from the base checkout with `cargo build --release --locked -p unrepresentable-policy`.                                                                                                                                                                                                                                                                                                                                                                        |
| `policy/src/render`                                                                           | The one function that renders every PR-derived string, and the set of invisible, bidirectional, filler, variation-selector and tag characters that make a path unsafe (`has_unsafe_chars`).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `policy/src/classify`, `data`, `artifacts`                                                    | Path classification, lint and dependency diffs, zizmor mapping; stored verdict data; strict line format (and per-job allow-list of reason codes) for result artifacts.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| `policy/src/approval`, `codeowners`                                                           | Approval conditions and CODEOWNERS matching.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `policy/src/report`, `verdict`, `checkrun`                                                    | Comment and check content, verdict, merge message and verdict block, check-run read/write.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `policy/src/post`, `review`, `audit`                                                          | The `policy-evaluate`, review/sweep and audit jobs.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `policy/src/pushskip`                                                                         | `push-skip`: decides whether a push to the default branch can skip its checks (see below).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `policy/src/releasegate`                                                                      | `release-gate <commit>`: the release workflow's green-CI gate; it exits 1 unless CI for the commit succeeded (see "Policy crate notes").                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `policy/src/fetchhead.rs`, `selflint`, `differential`, `headjobs`, `installtool`, `installer` | Job helpers.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `policy/src/github.rs`, `process.rs`, `io.rs`                                                 | `gh api` client, the only process-spawning module, the only file/environment/stdio module.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `policy/sandbox.sh`, `offline.sh`, `stage-sandbox.sh`, `take-file.sh`                         | The sandbox for head code and head data: `sandbox.sh` runs a command as uid 65534 (or 65533, see below) with no network (only a loopback interface, brought up before privileges are dropped), in a PID namespace of its own, with no `sudo` (`no_new_privs`) and an empty environment, and kills and checks for leftover processes of that uid before it returns; `offline.sh` is the same with a staged directory required; `stage-sandbox.sh` makes that directory (root-owned toolchain copy, scripts and tools; a copy of the head and the crates owned by the sandbox user); `take-file.sh` opens one file once and copies it out of it, refusing a link, a FIFO or other file that is not regular (checked before the open, so a FIFO cannot block it; the whole script also runs under `timeout 120`), a file that leads outside the sandbox directory, and one over 256 MiB. |
| `policy/settings.json`, `corpus.lock`, `tools.lock`                                           | Expected settings; public corpus pins; pinned tool archives (zizmor, cargo-deny, cargo-mutants).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| `ci/unrepresentable.sh`, `ci/README.md`                                                       | Consumer installer template (filled in by `unrepresentable-policy make-installer`); the maintainer description of `ci/`. Adopters read `docs/ci.md`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `ci/cargo.sh`                                                                                 | The cargo entry point of the workflows.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `ci/checks.sh`, `ci/version.sh`                                                               | The list of checks and a version reader; see `ci/README.md` and the script headers.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `policy/src/**/tests.rs`                                                                      | `cargo test -p unrepresentable-policy`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |

## Classes of change

- auto: non-protected docs only (`README.md`, `NOTES.md`, `CHANGELOG.md`,
  `docs/**/*.md`). Nothing under `src/` is auto any more (see
  `RULES_CODE_CHANGED` below).
- append-only: newly added files under `tests/fixtures/`, `tests/expected/`,
  `tests/canaries/`, `canaries/rust/src/bin/`. No reason by themselves.
- human: everything else. Every file under `src/` (all of it: `src/scan/**`,
  `src/rules.rs`, `src/lintcov.rs`, `src/against.rs`, any extension,
  case-insensitive; `RULES_CODE_CHANGED`), any modification or deletion under an
  append-only directory (`JUDGE_MODIFIED`), `.github/**`, `Cargo.toml`,
  `Cargo.lock`, toolchain and clippy files, everything under `.unrepresentable/`
  (`config`, `ledger.md`, `canary-exemptions.md`; `config` is
  `CONFIG_FILE_CHANGED`, the rest `LEDGER_FILE_CHANGED`), `policy/**` (this
  includes the crate source in `policy/src/`), `ci/**`, `tests/**` other than
  append-only additions, CODEOWNERS, LICENSE, LICENSE-MIT, LICENSE-APACHE,
  SECURITY.md, symlinks and submodules, any path with control, bidi or
  zero-width characters, and any path the policy does not recognise (default
  deny, `OTHER_PATH_CHANGED`). Matching is case-insensitive.

`policy/**` stays in the judge class on purpose. The crate decides what
auto-merges, so a change to it must reach a human whatever else it touches; as
`code` it would let a change under `check --against` with no approval (exit 0)
alter the merge policy. The consequence is that one change cannot touch
`policy/**` and `src/**` or `tests/*.rs` together (`judge-code-pairing`); split
it. Judge classification is only about change control: the crate is still
scanned by every rule, linted under the workspace table and tested by
`cargo test -p unrepresentable-policy`, because members are scanned whatever
`code` and `judge` say. `policy/Cargo.toml` also matches the built-in
`**/Cargo.toml` judge glob. `.unrepresentable/config` classifies as
`CONFIG_FILE_CHANGED` and any other `.unrepresentable/` path as
`LEDGER_FILE_CHANGED`.

`RULES_CODE_CHANGED` exists because the differential check cannot examine
everything (see "Known limits"): a head binary can recognise the comparison and
behave differently, or carry the base rules inside itself, and no black-box
comparison finds that. So a change to the code that enforces the rules is read
by a person, in this repository's own merge policy, and the approval path is
CODEOWNERS like every other protected class (an owner of the changed path, here
`src/`, who did not write the change). Tests, fixtures and docs keep their
classes: a docs-only change is auto, an added fixture is append-only, other test
code is `TEST_CODE_CHANGED`. The differential and the mutation tests remain as
checks for a change that loses findings by mistake.

Reason codes: `JUDGE_MODIFIED`, `LINT_LOWERED`, `DEPENDENCY_CHANGED`,
`WORKFLOW_CHANGED`, `UNSAFE_PATH_CHARS`, `MANIFEST_CHANGED`,
`TOOLCHAIN_CHANGED`, `CONFIG_FILE_CHANGED`, `LEDGER_FILE_CHANGED`,
`POLICY_CHANGED`, `CODEOWNERS_CHANGED`, `PROTECTED_DOC_CHANGED`,
`TEST_CODE_CHANGED`, `RULES_CODE_CHANGED`, `SYMLINK_OR_SUBMODULE`,
`OTHER_PATH_CHANGED`, `ZIZMOR_FINDING`, `DIFF_UNPARSABLE` (classify);
`CONFIG_LOOSENED`, `LEDGER_ROW_ADDED`, `BASELINE_GREW`, `TOOL_APPROVAL`
(self-lint); `LOST_FINDING` (differential); `LOST_FINDING_HIDDEN`
(differential-hidden); `MUTANT_SURVIVED` (mutants). Blockers that cannot be
approved: `INPUT_JOB_FAILED`, `RESULT_UNPARSABLE`, `SETTINGS_AUDIT`.

Verdict is `auto` only with no reasons and no blockers. Otherwise `needs-human`;
with a blocker it is also not approvable. A failed `head-checks` or `self-lint`
is a blocker.

Approval (policy-review.yml and policy-sweep.yml), all of: the reviewer's latest
effective review is APPROVED on a commit equal to the current head; the reviewer
is a code owner (BASE branch CODEOWNERS) of every path-based reason, and of
`policy/**` for non-path reasons (`LOST_FINDING`, `LOST_FINDING_HIDDEN`,
`MUTANT_SURVIVED`, `CONFIG_LOOSENED`, `LEDGER_ROW_ADDED`, `BASELINE_GREW`,
`TOOL_APPROVAL`, `UNSAFE_PATH_CHARS`); the reviewer is not the PR author, did
not author or commit any commit of the PR (read from `GET /pulls/N/commits`: a
code owner who pushes to someone else's pull request cannot then approve the
result), and is not a bot; all input jobs of the policy run that produced the
verdict succeeded (re-read through the jobs API). Comments such as `/approve`
are never read. Humans merge approved PRs themselves; the App merges only `auto`
PRs.

Re-evaluation also compares the stored verdict with the world as it is now. The
verdict records the base commit and the policy commit it was made with; `review`
reads the tip of the default branch and the commit of the policy code it runs,
and when either differs it does not apply the approval (status `stale`). A check
that already says success is turned back to failure, so an old success cannot
stand on a base that moved. A new evaluation is needed: push to the pull request
(for example "Update branch") or close and reopen it. Re-running the old
workflow run does not help, because a re-run keeps the old base commit. Listings
read through the API (`paged`) fail when they reach their 2000-item cap or a
page is not a list, and the sweep reads every open pull request, so a decision
is not made on part of a list.

The verdict data used for re-evaluation is stored in the App-authored check run
(`output.text`, a `policy-data:` line). Nobody else can write a check run with
the App's identity, so nothing a PR can create is read back as trusted.

## Setup humans must do outside the repository

1. Create the GitHub App `unrepresentable-policy` (settings > Developer
   settings). Permissions exactly: Checks write, Pull requests write, Contents
   write (merge only), Metadata read. The workflows mint tokens narrower than
   that: the decision, review and sweep tokens get checks write, pull requests
   write and contents read; only the merge step of `policy-evaluate` mints a
   second token with contents write. No webhook needed. Install it on this
   repository only. Generate a private key.
2. Environments (Settings > Environments):
   - `policy`: deployment branches = one custom branch policy naming exactly the
     default branch (not the "protected branches" option, which admits every
     protected branch; the audit treats that as drift). Secret
     `POLICY_APP_PRIVATE_KEY` (the key); variable `POLICY_APP_ID`. Used only by
     jobs that run base code and never build or run head code
     (`policy-evaluate`, `review`, `sweep`).
   - `hidden-corpus`: default branch only. Secrets `HIDDEN_CORPUS_URL` (https
     tarball URL) and `HIDDEN_CORPUS_TOKEN`. Used only by `differential-hidden`,
     which never fetches or reads the head and runs both binaries only inside
     `sandbox.sh`.
   - `audit`: default branch only. Secret `AUDIT_TOKEN`: a fine-grained,
     read-only token with Administration (read), Metadata (read), Contents
     (read), Actions (read). No PR job uses this environment.
   - `release`: deployment tags `v*` only (one tag policy with the pattern
     `v*`), and **required reviewers are mandatory**: a crates.io publish cannot
     be undone, and `assemble` and `publish-crate` both run in this environment,
     so the reviewers approve each release before anything is created or
     published. `settings-audit` fails when the environment has no required
     reviewer rule that names someone.
3. Actions settings: default `GITHUB_TOKEN` read-only; "Allow GitHub Actions to
   create and approve pull requests" off; fork pull request workflows require
   approval from all outside collaborators.
4. Repository settings: squash merging only (disable merge commits and rebase);
   native auto-merge disabled.
5. Rulesets (Settings > Rules), matching `policy/settings.json`:
   - `default-branch` on the default branch: require pull request with exactly
     the parameters in `settings.json` (squash only, dismiss stale approvals on
     push, no native last-push approval, no required approving review count, no
     code owner review, no thread resolution: the `policy` check does the
     approving, and native required reviews would stop the automatic merge of a
     pull request that needs no human), require status check `policy` with the
     expected source set to `unrepresentable-policy` (never "any source") and
     "require branches to be up to date" on, require linear history, block force
     pushes, block deletion. The only bypass actor is the Admin role (role id 5)
     in bypass mode `pull_request`. An admin MAY merge a pull request although
     rules are unmet (the `policy` check and the approval included), and MUST do
     it through a pull request: the mode `pull_request` gives no bypass for a
     direct push to the default branch (never use mode `always` here), and the
     rule still requires a pull request for every change. The owner is the sole
     maintainer and code owner and cannot approve their own change, and accepts
     that single-maintainer changes to protected paths are approved by the owner
     through this bypass. The approval rules still apply to the policy check
     itself and to every contributor who is not an admin. The audit compares the
     complete parameter object of the pull request rule and of the status check
     rule: a parameter GitHub adds or a value someone changes is reported as
     drift until `settings.json` lists it.
   - `branch-creation` on all branches: restrict creation; bypass only the
     Maintain and Admin roles (role ids 2 and 5), in bypass mode `always`.
   - `release-tags` on `refs/tags/v*`: restrict creation, update and deletion;
     bypass only the Maintain and Admin roles (role ids 2 and 5), in bypass mode
     `always`. The bypass list must contain a role the repository owner actually
     holds. The owner of a personal-account repository is an admin (role id 5),
     not a maintainer, and a ruleset that lets only Maintain bypass would lock
     the owner out of creating the first tag. Verify it in the scratch
     repository before using the setup for real: (1) as the owner, push a tag
     `v0.0.0-test` that matches `v*` (it must succeed through the bypass, and be
     deleted again through it); (2) as a user with write access but neither
     Maintain nor Admin, the same push must be refused; (3) as the owner, create
     a branch directly in the repository (it must succeed); (4) run
     `settings-audit` and confirm it reports no drift for the actor lists (the
     audit compares the bypass mode of every actor too, and reports a missing
     actor, an extra actor and a different mode). If the owner is locked out,
     the list in `settings.json` and the ruleset are wrong together: fix both,
     never only the ruleset. The native "require approval of the most recent
     push" rule (`require_last_push_approval`) is not used. With zero required
     approving reviews it can make GitHub refuse the App's merge of an `auto`
     pull request, so it is false in `settings.json` and the audit pins that.
     What the policy check does instead, and what it does not: the approval must
     be of the exact head commit, `dismiss_stale_reviews_on_push` stays on, and
     a reviewer who is the pull request author, or the author or committer of
     any commit of the pull request (`GET /pulls/N/commits`), is not counted.
     The rule is not the same as last-push approval. The rule reads who is named
     on the commits, and a pusher is not named on a commit. So when the names
     cannot be trusted, nothing is approvable: if any commit has no login for
     its author or for its committer (its email is linked to no GitHub account,
     so the API gives `null`), or if the number of commits listed differs from
     the `commits` count of the pull request (the API lists at most 250 and
     stops silently), the result is `CommitsNotAttributable`, and the check run
     says to link the email to an account or to merge by hand. A committer
     `web-flow` (a commit made in the GitHub web editor) has a login and counts
     as attributable. What remains: a maintainer who pushes attributable commits
     to someone else's pull request is excluded as an approver, so with a single
     code owner such a pull request needs a second code owner, or the owner
     re-opens the change as a pull request of their own, or the owner merges it
     as an admin through the pull request bypass of the default branch ruleset;
     and a pull request with unattributable commits can only be merged by hand.
     Also remaining: git identities are not verified and signed commits are not
     required, so a pusher who writes someone else's linked email into both
     fields of a commit is not excluded as an approver. Pull request rule
     parameters are written as the API names them (`require_code_owner_review`,
     not "reviews").
6. crates.io, and the first publish. Trusted publishing can only be added to a
   crate that already exists, and `publish-crate` runs after `assemble` has
   created the GitHub release, so the first release cannot publish itself. First
   publish of the crate (once, by hand):
   1. On a clean checkout of the commit that will be tagged (the default branch,
      version in `Cargo.toml` equal to the tag), the `package` check
      (`sh ci/checks.sh package`) can be run to test the package.
   2. Create a crates.io API token scoped to this crate name only with only the
      `publish-new` scope (a token for a new crate name has to be able to create
      it), set to expire in a day, and run `cargo publish --locked` for
      `0.0.1-alpha.1` by hand with it, BEFORE tagging. Delete the token
      afterwards.
   3. On crates.io, add this repository, workflow `release.yml` and environment
      `release` as a trusted publisher of the crate. No long-lived token is
      stored anywhere after that.
   4. Tag `v0.0.1-alpha.1`. The release workflow now builds and creates the
      GitHub release (as a pre-release, with `--prerelease`, because the version
      has a suffix), and `publish-crate` finds that `0.0.1-alpha.1` already
      exists on crates.io and fails at `cargo publish`. The `cargo publish`
      failure is expected for this one release, and the release and its binaries
      are already complete: the crate on crates.io was published by hand from
      the same commit (check that its files equal `cargo package` of the tag if
      you want to be sure). Do not re-run it. From the second release on,
      publishing is tag-triggered and needs no manual step. If `publish-crate`
      fails after the release exists (crates.io outage, an expired trusted
      publisher, a reviewer who did not approve in time), re-run only the
      `publish-crate` job (Re-run failed jobs, from the run page): it publishes
      the same tagged commit without building, and the release and its binaries
      are not created again. `publish-crate` deliberately has no condition that
      skips it when the version is already on crates.io: a skip would accept a
      crate published from somewhere else as this release's, and it would weaken
      the gate, so it is documented instead.
7. Run `settings-audit` once by hand (workflow_dispatch). `policy-evaluate`
   refuses to merge and blocks approval unless the latest audit succeeded within
   24 hours.
8. Replace `@wtanaka` in `.github/CODEOWNERS` if maintainers change. Teams are
   not supported (the App has no organisation permission to resolve membership).

Bootstrap notes:

- The first merge of these workflows needs a human with bypass, because the
  required check does not exist until they are on the default branch.
- The policy overlays the head's tool files (`.unrepresentable/config`,
  `ledger.md`, `canary-exemptions.md`) onto the base checkout and runs the base
  binary on them. A pull request that moves those files to a new location is
  evaluated by a base binary that still looks at the old one, so `self-lint` and
  `differential` cannot evaluate it. Merge such a change by hand; it needs human
  approval anyway (`JUDGE_CHANGED`, `CONFIG_FILE_CHANGED`). Later changes are
  unaffected. A consumer repository must move its three files when it upgrades
  across such a change.

## Policy crate notes

- `unrepresentable-policy` is a workspace member under `policy/`, role `shell`,
  with `[lints] workspace = true`. It is not in `default-members`; workflows
  name it with `-p unrepresentable-policy`.
- Dependencies, all exact-pinned (`=x.y.z`), `default-features = false`, and
  checked by `cargo-deny` (`deny.toml`): `toml =1.1.6` (features `parse`, `std`)
  reads `Cargo.toml` lint tables and `Cargo.lock` packages in
  `classify/manifest.rs` through `policy/src/toml`; `serde_json =1.0.151`
  (`std`; untyped values only, no derive macros, no `preserve_order`) reads the
  GitHub API answers, the verdict data and the zizmor report through
  `policy/src/json`; `glob =0.3.4` matches CODEOWNERS patterns in `codeowners`;
  `base64 =0.23.1` (`alloc`) encodes and decodes the check run data, the
  artifact fields and the fetch credential in `bytes`. The crates named above
  are third-party code that runs with the App's trust: this is why
  `policy-evaluate` runs the policy tool built from the base checkout and why a
  head change to `Cargo.toml`, `Cargo.lock` or `deny.toml` is a human decision.
  The crates replaced a reader written here for each format. The reasons: the
  earlier `Cargo.toml` reader looked at single lines and treated any line it did
  not understand as "not a lint", so a lint could be hidden by a spelling it
  skipped; the crates read the whole document, and a document that does not
  parse is now an error, not an empty result. The JSON writer (`compact`) stays
  here: it sorts keys and escapes in one fixed way so that stored and recomputed
  verdict text are byte-identical.
- CODEOWNERS patterns are turned into `glob` patterns (one or two per rule) with
  the options `require_literal_separator` and case sensitivity, and keep the
  earlier tested meaning: a leading or inner `/` anchors, a trailing `/` means
  everything below a directory (not the directory itself), a name without a
  trailing `/` matches the file or directory and everything below it, `**/` and
  `/**` as whole segments, and the last match applies. Two spellings differ from
  before, deliberately: `**` inside a name (`a**b`) is read as `*`, and `[` is a
  literal character, not the start of a character class. A pattern that cannot
  be turned into a `glob` pattern fails the parse (exit 2) instead of silently
  matching nothing, because an ignored rule would hand ownership to an earlier,
  broader rule.
- Spawning is confined to `policy/src/process.rs`: `git`, `gh api` (the token
  goes through `GH_TOKEN` in the child's environment, never argv), `curl` (URL
  and bearer token on standard input as a curl config), `tar`, `chmod`,
  `mktemp`, `sha256sum` or `shasum`. Children have no timeout of their own; the
  job `timeout-minutes` cover it.
- No regular expressions and no zip, tar or sha libraries. Archives are listed
  and checked with `tar -t` and `tar -tv` before `tar -x`: plain files and
  directories only, no `..`, no absolute names, a 512 MiB cap. Digests come from
  `sha256sum`.
- `render` treats U+FFFD (what non-UTF-8 path bytes decode to) as unsafe,
  because Rust strings cannot hold lone surrogates.
- Verdict data in the check run is parsed strictly
  (`data::VerdictData::from_json`); damaged data reads as "no verdict data for
  this head".
- `make-installer` takes a `VERSION` of exactly `N.N.N` or `N.N.N-PRERELEASE`
  (no leading zeros in the numbers; dot-separated identifiers of `[0-9A-Za-z-]`,
  none empty, numeric ones without leading zeros; no `+build` metadata, no `v`).
- `install-tool` accepts a `tools.lock` line only when its URL is exactly
  `https://github.com/<owner>/<repo>/releases/download/<tag>/<asset>`, the tag
  is the version (with or without `v`) and the asset is
  `<name>-<platform>.tar.gz` or `<name>-<version>-<platform>.tar.gz`, so a
  bumped version cannot leave an old or different URL behind. Its digest check
  is unchanged.
- The zizmor report is read as data: output that is empty, not JSON or not an
  array fails `classify`, which the decision reads as `INPUT_JOB_FAILED`, a
  blocker (the workflow runs zizmor with `|| true` because it exits non-zero on
  findings, so a crash looks like empty output).
- `render` also treats the invisible fillers (U+034F, U+115F, U+1160, U+3164,
  U+FFA0), the Mongolian free variation selectors (U+180B to U+180E), the
  variation selectors (U+FE00 to U+FE0F and U+E0100 to U+E01EF) and the
  invisible format controls (U+2065, U+206A to U+206F) as unsafe, in
  `has_unsafe_chars` as well as in rendering.
- `.unrepresentable/config` lists the member's `ambient` entries (file,
  environment, time and stdio only in `io.rs`; processes only in `process.rs`
  and `main.rs`) and its newtype method lists.
- Tests are unit tests under `policy/src/**/tests.rs`. `workflow_checks.rs`
  reads `../.github/workflows` (cargo runs tests from the package directory);
  `testkit.rs` has a fake GitHub for the decide, merge and review flows.
- `release-gate <commit>` reads `GH_TOKEN`, `GITHUB_REPOSITORY` and
  `DEFAULT_BRANCH` and uses the same client. It lists the `policy-pr` workflow
  runs of the commit (`actions: read`), a run of event `push` or
  `workflow_dispatch` of the default branch, from the workflow file of the
  default branch, then the commit's check runs (`checks: read`). A manual run is
  accepted because a push run can be dropped (a newer push replaces the pending
  run of the concurrency group, even with `cancel-in-progress: false`), after
  which the gate could never open for that commit; to release such a commit, run
  `policy-pr` by hand on the default branch (Actions > policy-pr > Run workflow)
  and re-run the release. A manual run skips `skip-check`, so it opens the gate
  only through the first rule below. The gate exits 0 only if each of the nine
  push jobs has a successful check run from the Actions app in such a run, or if
  the jobs were skipped, `skip-check` succeeded, no job failed or is pending,
  and the `push-skip` decision for the commit is yes (the same code,
  `pushskip::judge`). A skipped run has no advisory check of its own, so
  `prepare` in `release.yml` runs `deny-advisories` (pinned cargo-deny, network,
  no secret) for every release, so every release has a fresh advisory check even
  when its push run was skipped. Every other outcome, including a missing token
  or an API error, exits 1 and prints one line that says to re-run the release
  workflow after CI is green. The `verify` job of `release.yml` builds the tool
  from the tagged tree, which the ancestry check shows is on the default branch,
  so it is trusted; the job has `actions`, `checks`, `pull-requests` and
  `contents` read access only.
- What `push-skip` and `release-gate` rely on, exactly: a skipped push run is
  justified by an App-authored `policy` check run for a pull request head with
  the identical git tree. The stored check run proves what the pull request path
  proves, and no more: `classify`, `self-lint`, `differential`,
  `differential-hidden` and `mutants` ran with the reasons in the stored
  verdict, and a human approved those reasons. The `head-checks` result (fmt,
  clippy, tests, canaries and the rest) is head code reporting about itself in a
  sandbox, so a skipped push run means "a pull request with this tree was judged
  by base code", not "the tests were run by trusted code on this tree". For a
  tree whose `src/` differs from the base this is now always a human-approved
  change (`RULES_CODE_CHANGED`); for a docs-only tree, nothing executable
  changed. A push run that is not skipped runs every check on the pushed tree by
  trusted code, and `release-gate` also needs that, or the skip evidence above,
  or a manual run.
- `push-skip` reads `GH_TOKEN` and `GITHUB_REPOSITORY`, queries the API through
  the same `gh api` client as the other subcommands (so `gh` is still spawned
  only through `process.rs`), and prints `skip=yes|no` and a one-line `note=`
  for `$GITHUB_OUTPUT`.
- Not exercised locally: every workflow, the real `gh` calls, `curl` downloads,
  the `sandbox.sh` and `offline.sh` wrappers, and the `differential` and
  `install-tool` network paths. Their pure logic and a fake GitHub are tested;
  the scratch-repository test plan below covers the rest.

## Known limits

- `classify` compares paths case-insensitively with `to_lowercase` plus the few
  characters whose full case folding differs (`ſ`, `ß`, the `ﬀ` to `ﬆ`
  ligatures). There is no Unicode normalisation (NFC), because that needs tables
  or a dependency. Only non-ASCII names are affected, and an unknown path needs
  a human anyway, so this can change a reason code but cannot auto-merge
  anything.
- `differential run` compares the findings of the base binary and the head
  binary on inputs, each in a workspace of its own
  (`policy/src/differential/workspace.rs`). Inputs come in three kinds:
  `--head-input` (data from the pull request, such as its `tests/fixtures`),
  `--input` (the pinned public corpus) and `--hidden-input` (the secret corpus;
  it cannot be combined with any other kind, because it is compared in a run of
  its own, `differential-hidden`). An input from the pull request is never used
  as it is, even when it has `.unrepresentable/config` of its own: the workspace
  is always generated, and `.cargo`, `.unrepresentable`, `build.rs`,
  `rust-toolchain*` and `Cargo.*` are removed from the copy, because they are
  how a pull request would run code in whatever evaluates it (a `rustc-wrapper`,
  a toolchain by path, a build script). A public or hidden input that has its
  own config is used as it is. The generated workspace has the config (a
  forbidden baseline, `default-range = "HEAD"`, `code = ["crates/**"]` because
  the tool requires `code`, every member pure), an empty ledger and
  canary-exemptions file, a `Cargo.lock`, and a git repository with one commit;
  each run passes `--range HEAD`, because the tool exits 2 when it gets no range
  and a range read from the config selects no change (`HEAD` is an explicit
  command-line range, which may be empty). The corpus is scanned, not built:
  each crate found in the input (a directory with `Cargo.toml` and `src/lib.rs`
  or `src/main.rs`) becomes a member with a generated manifest that has no
  dependencies, and an input with no crate becomes one member whose `src/lib.rs`
  names every `.rs` file with `#[path]`. The lock is fixed text for those
  manifests, so no network is used. BOTH binaries run through the sandbox prefix
  (`--prefix` is required), each in its own writable scratch build directory
  with its own `HOME` and `CARGO_HOME` inside it (passed as `env NAME=VALUE`
  words, since the sandbox clears the environment), and the child also gets a
  `safe.directory` entry for the workspace through `GIT_CONFIG_*`. What remains:
  the manifests, features and dependencies of the corpus crates are not seen, so
  only the findings of the scan and of lint coverage on the generated workspace
  are compared; the move/rename and approval checks report no change at all.
- What the differential can and cannot prove. The differential is a black-box
  comparison: it shows that the head binary reports every finding the base
  binary reports on the inputs it was given. The differential cannot show that
  the head binary would report them anywhere else. A head binary that recognises
  the comparison (the generated workspace, `check --range HEAD`, a scratch
  `CARGO_TARGET_DIR`, an input name) and then behaves like the old rules there,
  or carries the old rules inside itself, loses nothing on the comparison and
  still weakens the tool on real code. Running the two binaries as different
  users in private directories (see "What each job is allowed to do") stops the
  simple forms of this (calling the base binary, or leaving it a file), not the
  form that embeds the base rules, because the rules are public and a pull
  request can copy them. That is the reason a change under `src/` is a human
  decision here (`RULES_CODE_CHANGED`) and the comparison, like the mutation
  tests, detects a change that loses findings by mistake and does not defend
  against a hostile one. Reviewers of such a change should read what it does
  when it is handed an input it has not seen before, and look for any use of the
  environment, the path of the input, the arguments or the time.
- A differential run cannot pass on empty input. A binary that exits with
  anything but 0 or 1 on an input, or dies from a signal, makes that input an
  error whatever the other binary did. The base binary's findings are read from
  the lines `path:line:col: rule: message` of its standard output; a line that
  is none of that, a blank line, `range:` or `approval:` is an error, and so is
  a base run with fewer findings than the input must give (constants in
  `differential/mod.rs`: 100 for the fixtures, one for the public and hidden
  corpora, one for any other name; the fixtures give about a thousand). An empty
  corpus, an unpacking that produced nothing, or a change of the output format
  is therefore an error, not "nothing was lost".
- Hidden corpus: kept outside the repository and fetched by
  `differential fetch-hidden` from `HIDDEN_CORPUS_URL` in the
  `differential-hidden` job. The base and head binaries run over it, so a head
  binary that weakens a rule only on inputs outside the fixtures, the canaries,
  the public corpus (`corpus.lock`: `syn` and `proc-macro2`) and the hidden
  corpus is not caught. Findings lost on it are reported only as a number
  (`LOST_FINDING_HIDDEN`: "N findings lost on the hidden corpus"), never as a
  path, rule or message, and its size is not reported, so a pull request cannot
  read the corpus out of the comment, the check output or the annotations. What
  a pull request can still learn is the number of findings its own binary lost,
  which is as much as it can send by choosing what the head binary prints; the
  head binary has no network and no way to write outside its sandbox.

## What each job is allowed to do

- `classify`: base code; the head is fetched into `head/` as files and read
  through `git show`/`git archive`; nothing from `head/` is built or executed
  (the base binary reads it as data, and the pinned zizmor reads the workflows
  as data).
- `self-lint`: the base binary runs over the head tree, and the base binary runs
  `cargo clippy`, so a head `build.rs`, `.cargo/config.toml` (`rustc-wrapper`),
  `rust-toolchain.toml` (a toolchain by path) or procedural macro would run. It
  therefore runs as an unprivileged user (uid 65534, no `sudo`, `no_new_privs`)
  with no network and no token, through `offline.sh`, on a read-only copy of the
  head (and, for the first run, of a one-commit repository of the head tree with
  the base config and ledgers overlaid), with a scratch build directory and a
  toolchain called directly (not through rustup, so a head `rust-toolchain` file
  is not read). The head's crates come from its own lock when that is
  crates.io-only with checksums, fetched earlier with the base checkout as the
  working directory; a head with git or path dependencies in its lock has no
  crates, so the run fails and the job blocks (the same as `head-checks`).
- `build-base`: base code at the base commit only (`ref` is `github.sha`, and
  the job checks `HEAD` equals it). It builds `unrepresentable-policy` and
  `unrepresentable` once, uploads each as an artifact and passes the sha256 of
  each as a job output. No other job in the pull request path compiles base
  code. Every consumer (classify, self-lint, head-checks, build-head,
  differential, differential-hidden, mutants, policy-evaluate) downloads the
  artifact into `base/target/release`, checks it against the job output with
  `sha256sum -c`, and only then makes it executable, so a forged or damaged
  artifact stops the job. All checkouts in the pull request path use the same
  `ref: github.sha`.
- `head-checks`, `build-head`, `mutants`: head code; `contents: read`, no
  environment, no secrets, credentials not persisted. Crates are fetched with
  network allowed, as the runner user, in a step whose working directory is the
  base checkout (cargo reads its configuration from the working directory and
  its parents, so the head's `.cargo/config.toml` is never read;
  `--manifest-path` points at the head's manifest, which is only parsed): the
  base lock, plus the head lock only if it is crates.io-only with checksums.
  Then everything the head can influence runs as uid 65534 with no network
  (`offline.sh`) in `/opt/sandbox`, made by `stage-sandbox.sh`: a root-owned
  copy of the toolchain called directly, the scripts and tools from the base
  checkout (root-owned), and a copy of the head and of the crates that the
  sandbox user owns. The head code cannot write the base checkout (including the
  sha256-verified policy binary), `$RUNNER_TEMP`, `$GITHUB_ENV` or the status
  and exit files (the outer shell writes those), and the head code has no
  `sudo`. The one file taken out of the sandbox, the head binary, goes through
  `take-file.sh`, which opens it once and checks the open file (inside the
  sandbox directory, regular, at most 256 MiB) instead of checking a path and
  copying the path later; the mutants lists are read by the policy tool, which
  refuses a link and a file that is not a plain file. Results are necessary, not
  sufficient: head code can write false results, so the decision rests on
  `classify`, `self-lint` and `differential`.
- `differential`: base orchestration. The head binary (an artifact) and the base
  binary run through `sandbox.sh` (no network, empty environment) as different
  users: the base binary as uid 65534, the head binary as uid 65533. Each is
  installed (`stage-sandbox.sh --private-bin`) into a directory that only its
  own user can search (`/opt/sandbox/private-65534`,
  `/opt/sandbox/private-65533`, mode 0750, group the user, not on any `PATH`),
  so the head binary can neither read nor run the base binary, and the runner's
  own copy of it (in the checkout) is hidden from both users by
  `chmod 0700 "$GITHUB_WORKSPACE"`. Each run of each binary on each input has a
  scratch directory of its own (`HOME`, `TMPDIR`, `CARGO_HOME` and
  `CARGO_TARGET_DIR` inside it) made fresh by the policy tool, given by
  `sandbox.sh` to that run's user and mode 0700, so the head binary cannot leave
  a file (a `cargo-home/config.toml`, say) for the base binary's next run. The
  head's fixtures and the public corpus are its inputs; it has no environment
  and no secret.
- `differential-hidden`: the only job with the `hidden-corpus` environment. It
  does not fetch or read the head; its only head input is the head binary, which
  runs through `sandbox.sh`. The secret is in the environment of the one step
  that downloads the corpus (base code), which ends before any head code runs.
- `policy-evaluate`: base code, environment `policy`. It runs the policy tool
  built by `build-base` (checked by sha256) and runs no cargo, make or script
  from the head directory (it does not even fetch it). It mints the App token
  for the decision with checks write, pull requests write and contents read, and
  a second token with contents write only in the step that merges, so a bug
  before the merge cannot write contents. `policy-review.yml`,
  `policy-sweep.yml` and `settings-audit.yml` compile nothing where their secret
  is: a `build` job with no environment compiles the tool and hands it over by
  sha256.
- `head-checks` stays one job. Splitting it would not weaken the sandbox, but
  each part would repeat the toolchain install, the crate fetch and the build
  (the checks share one build directory), and the result artifact and the
  approval re-check (`INPUT_JOBS`, read back by job name) are keyed to one
  `head-checks` job, so splitting would change the verdict data. The list of
  checks it runs is `ci/checks.sh` from the base checkout (copied into the
  sandbox directory), run inside `offline.sh`, except `audit-workflows`, which
  runs the pinned zizmor outside the sandbox because it only reads the head's
  workflow files as data. `deny` (cargo-deny licenses, bans, sources) is in the
  list and runs offline: the pinned cargo-deny is installed in the step that has
  the network and staged into the sandbox. The `head-checks` result passes only
  when every check in `EXPECTED_CHECKS` (`policy/src/headjobs/mod.rs`, tested to
  equal the workflow and `ci/checks.sh`) is in the status file exactly once with
  exit 0 and nothing else is: a missing, repeated, unknown, failed or garbled
  line, or a status file that cannot be read, fails the job. `mutants` fails
  when `cargo mutants` exits 2 or 3 and the list it promises is missing or
  empty, and when a list cannot be read or is not a plain file. `owntree` runs
  over `$BASE_SHA...HEAD`. `deny-advisories` is not in the list: it needs the
  advisory database, so it runs only in the push jobs and the hourly sweep.
- `skip-check` and the other push jobs (`push`, `workflow_dispatch`): the pushed
  tree is the default branch, so it is trusted. These jobs have `contents: read`
  (`skip-check` also `checks: read` and `pull-requests: read`), no environment
  and no secrets, and do not run on `pull_request_target`. `skip-check` asks the
  API whether a pull request head with the same git tree already has a
  successful App-owned `policy` check run; any doubt or error prints `skip=no`,
  and the later jobs run whenever the output is not `yes`, even if `skip-check`
  failed or was skipped. A skipped push run therefore needs a positive,
  App-authored result for identical files. Skipping is sound only because the
  `head-checks`, `self-lint` and `mutants` results behind the `policy` check
  cannot be forged by head code: they are produced by an outer shell and base
  code from files a sandboxed, unprivileged process wrote as data. If head code
  could run as the runner user it could write `status.txt` and make
  `head-checks` look green for a tree that fails, and every later push of that
  tree would skip its checks. Changes to the sandbox must preserve this. The
  `release-gate` accepts a skipped run only when the same logic says yes, and it
  accepts only a `policy-pr` push run of the default branch itself.
- No caches in `policy.yml` or `policy-review.yml`.
- zizmor configuration: every zizmor run (`classify`, `head-checks`,
  `ci/checks.sh audit-workflows`) passes `--config` with `policy/zizmor.yml`
  from the base checkout (the pushed, trusted tree for push jobs). An explicit
  `--config` makes zizmor load that file only and discover none, so the head's
  own `.github/zizmor.yml` is never read. `policy/**` is a judge path, so a pull
  request cannot change the file without human approval. The file ignores
  exactly two `dangerous-triggers` findings, each at its `file:line:column`:
  `workflow_run` in `policy-review.yml` (4:1) and `pull_request_target` in
  `policy.yml` (28:1). Both are the design: the policy runs from the base branch
  and treats the pull request as data. Ignoring by position, not by file, means
  a new dangerous trigger anywhere else is still reported, and a change that
  moves either `on:` line is reported again; that is intended, and the entry is
  then moved by a reviewed change. A test pins the two entries and the two
  lines.
- No workflow writes `GITHUB_ENV` or `GITHUB_PATH` (zizmor `github-env`). The
  toolchain step validates the channel (`MAJOR.MINOR.PATCH`) and writes it as a
  step output; the steps that stage the sandbox get it through `env:`. Installed
  tools are put on `PATH` inline on the one command that needs them. No pull
  request controlled value reaches an environment file, and the sandbox
  environment is still built explicitly by `policy/sandbox.sh`.

- `policy-review.yml` and the sweep. `workflow_run` names no pull request for a
  pull request from a fork, so the review passes the head commit of the
  triggering run (`--head-sha`) and the tool finds the open pull requests with
  that head through `GET /commits/{sha}/pulls`; when that finds nothing, or
  fails, every open pull request is re-evaluated (never fewer). The hourly sweep
  re-evaluates every open pull request. In both, a pull request whose evaluation
  fails does not stop the others: the rest, including the downgrade of a stale
  success, are still evaluated, each failure is listed as
  `PR #n: could not be re-evaluated: ...`, and the job then fails.
- `default-range` (see
  [docs/configuration.md](../docs/configuration.md#default-range-is-not-a-trust-anchor)):
  the range a config names is a convenience for local use and is not trusted by
  the policy. Every CI call passes its own `--range` or `--against` to `check`;
  `workflow_checks.rs` pins that.
- Not done: the head-checks of one pull request share one working copy (see
  above); the shared `/tmp` between the two differential users; zizmor runs with
  `--config policy/zizmor.yml`, taken from its documentation (an explicit
  `--config` makes zizmor discover no other configuration); whether an Ubuntu
  runner has a process of uid 65534 or 65533 that `pkill` would kill and that
  systemd would restart.

## Trust assumptions that remain

What the pull request path still relies on:

- The sandbox is `sudo -n unshare --net --pid --fork --kill-child --mount-proc`
  and then `setpriv --reuid=65534 (or 65533) --no-new-privs`, with a loopback
  interface brought up first. The sandbox relies on the hosted Linux runner
  (passwordless `sudo` for the runner user, util-linux `unshare` and `setpriv`)
  and on the kernel's user, network and privilege separation. A kernel or runner
  escape is out of scope. None of it can be run on a development machine; it is
  described and tested statically (`workflow_checks.rs`) and its pure logic is
  unit tested.
- Leftover processes. Head code can start a process that outlives its command (a
  daemon, a double fork). Each sandboxed command therefore runs in a PID
  namespace of its own, so that the kernel ends everything in it when its first
  process ends, and `sandbox.sh` then also runs `sudo -n pkill -KILL -u UID` and
  requires `pgrep -u UID` to find nothing (it exits 2 otherwise) before it
  returns, so nothing of the sandbox user is running when the outer shell reads
  what the command wrote (`mutants.out`, the head binary, the status of a
  check). The wait makes `take-file.sh` safe against a swap while it reads, and
  the checks in the later steps safe against a process that rewrites
  `work/head/target/release/unrepresentable` behind them. Remaining: the checks
  of `head-checks` share one copy of the head and one target directory, one
  after the other, so head code in one check (a build script, a test) can change
  the files a later check uses. Giving every check a fresh copy would repeat the
  whole build and was not done; it does not matter for the decision, because
  `head-checks` is head code reporting about itself and the decision rests on
  `classify`, `self-lint` and `differential`, whose inputs it cannot write. The
  binary the differential uses is the one `build-head` took out right after the
  build, once, and it is never rebuilt or rewritten afterwards. `sandbox.sh`
  kills every process of the uid, so two sandboxed commands of one uid must not
  run at the same time (the jobs run them in sequence), and a runner that keeps
  a daemon of uid 65534 or 65533 of its own would make every sandboxed command
  fail.
- Network namespace: the sandbox has a loopback interface (`ip link set lo up`
  runs inside the new namespace before privileges are dropped), so tests that
  bind `127.0.0.1` behave as on a developer machine, and nothing else. Its
  loopback is its own: no other process can reach it, and it reaches no other.
- The two users of the differential share `/tmp`, `/dev/shm` and the other
  world-writable places of the runner. The policy tool points `TMPDIR` of each
  run into the run's own private directory, and neither binary needs the others,
  but a head binary can still leave files there; the base binary does not read
  them.
- Head code in the sandbox can read whatever uid 65534 may read: world-readable
  files on the runner, the staged toolchain, the head copy, the crates. The
  runner's own files that matter (the base checkout, `$RUNNER_TEMP`,
  `$GITHUB_ENV`, the status files) are not writable by it, and no secret is in a
  file: checkouts do not persist credentials, tokens live only in the
  environment of the steps that need them, and the sandbox starts from an empty
  environment. The sandbox can use CPU and disk until the job's
  `timeout-minutes`. Every result the sandbox writes can be false; that is why
  the decision rests on `classify`, `self-lint` and `differential`, whose inputs
  the sandbox cannot write.
- The head's manifests and lock are parsed by cargo as the runner user
  (`cargo fetch --manifest-path`, run from the base checkout, only when the head
  lock names crates.io with checksums). Parsing runs no head code, but a bug in
  cargo's parser would. The crates it downloads are the ones the lock names with
  their checksums; their build scripts run only later, in the sandbox.
- The toolchain copied into the sandbox is the one `rustup` installed from the
  channel in the base `rust-toolchain.toml`, trusted as rustup trusts it.
- `differential-hidden` and the head binary: a pull request can learn at most
  how many findings its own binary lost on the hidden corpus.
- The base binary, the policy tool and the pinned tools are built or downloaded
  by base code; the policy crate's own dependencies are third-party code (see
  "Policy crate notes").
- A pull request that adds a git or path dependency to the lock (or any lock
  that is not crates.io with checksums) gets no crates in the sandbox, so
  `self-lint` and `head-checks` fail and the policy blocks it; such a change is
  merged by hand.

## Test plan in a scratch repository

Create a private scratch repository with this tree, the App installed, and the
setup above. Open each pull request from a fork or a maintainer branch as noted
and check the result.

1. Workflow that posts a fake `policy` check run, commit status and comment (and
   prints a fake check mark): expect `WORKFLOW_CHANGED` and `ZIZMOR_FINDING`,
   `policy` failing, and the fake check ignored by the ruleset (source is the
   App).
2. Edit an existing fixture: `JUDGE_MODIFIED`, human.
3. Early `return` in a rule (no fixture covers it): `LOST_FINDING` from the
   differential and/or `MUTANT_SURVIVED`, annotated at the rule source.
4. Bump `syn` in Cargo.lock: `DEPENDENCY_CHANGED`, `MANIFEST_CHANGED`.
5. Remove a lint or lower one in `Cargo.toml`: `LINT_LOWERED` with "Approving
   lets ... warn instead of fail".
6. Add an `std::env::var` read: the tool's own rules fail
   `self-lint`/`head-checks` (blocker, not approvable).
7. File named with a newline and a fake check mark
   (`printf 'src/a\n✅ All clear.rs'`): `UNSAFE_PATH_CHARS`, comment shows an
   escaped inert span.
8. PR title and branch named `$(touch /tmp/pwned)` and `${{ secrets.X }}`:
   nothing executes; neither string appears in comment, check or logs of base
   jobs.
9. A PR that writes a poisoned cache or artifact: base jobs never restore
   caches; a forged artifact fails strict parsing (`RESULT_UNPARSABLE`).
10. Approvals by a non-owner, by the PR author, by a bot, and an approval made
    before the last push: none flips `policy` to success. Then an owner approves
    the current head: `policy` becomes success within the review workflow run.
11. Edit `review-signal.yml` (for example make it call something):
    `WORKFLOW_CHANGED`; confirm the PR's copy never runs with secrets.
12. Same-repo branch by a non-maintainer: branch creation is refused by the
    `branch-creation` ruleset; they must use a fork. A fork PR's workflows need
    approval before they run.
13. Library-only PR (a change in `src/` covered by tests, plus a doc line): the
    App merges it, squash, at exactly the evaluated commit; the commit message
    ends with the verdict block with empty reasons.
14. Push to a PR branch between evaluation and merge: the merge call fails
    because `sha` no longer matches; a new evaluation starts for the new head
    and the old comment is minimised.
15. Make the settings audit stale (disable the workflow for 25 hours, or change
    a setting): `policy-evaluate` blocks with `SETTINGS_AUDIT`; fix and rerun.

## Unit tests

`cargo test -p unrepresentable-policy` runs render tests (including the six
hostile strings), classification (also against a real git repository), the
artifact grammar, approval rules, report building, verdict and merge message,
the installer generator, the differential logic, fake-GitHub end-to-end runs of
decision, approval and merge, the settings audit, the `release-gate` decision
against the fake GitHub (all nine jobs succeeding passes; a failed, pending,
cancelled, neutral or missing job, jobs skipped without a push-skip yes, an API
error at any call, or a job from another app or workflow refuses), and the
review staleness rules, the whole-object audit of rule parameters and
environment deployment policies, and the `push-skip` decision against the fake
GitHub (an equal tree with a successful check skips; a different tree, a
missing, failed, unfinished or foreign check run, no pull request, or an API
error runs), and static checks over every workflow file (permissions, SHA
pinning, `persist-credentials`, no PR-controlled expressions in
`run`/`script`/`with`, no caches in the policy workflows, no Python left, every
job gated by `github.event_name`, push jobs run their checks when `skip-check`
fails, base binaries verified by sha256, every check name used in a workflow
exists in `ci/checks.sh`, the offline `head-checks` runs every check except the
network-only `deny-advisories`, every job that runs cargo-deny installs the
pinned one, and `checks.sh` calls no bare `cargo` and exits 2 for an unknown
name). The `toml`, `json`, `bytes` and `codeowners` modules have their own tests
(positions in error messages, source order, dotted keys and inline tables,
sorted keys, surrogate pairs, nesting depth, base64 vectors and strictness, glob
translation), and the classify tests cover lint changes however the tables are
spelled and manifests or locks that do not parse. The workflows themselves
cannot be run locally; see the test plan above.

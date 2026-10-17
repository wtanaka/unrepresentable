//! Settings audit: live repository settings and recent merge commits against
//! `policy/settings.json`. Read-only token (`AUDIT_TOKEN`). Prints one drift
//! per line (our own words; no PR text) and exits 1 on drift.

pub(crate) mod live;

#[cfg(test)]
mod tests;

use crate::json::{items_of, members_of, Flag, Json};
use crate::types::{Count, Name, Sha, Text};
use crate::verdict::parse_block;

/// One line of drift.
macro_rules! note {
    ($($arg:tt)*) => {
        Text::from_string(format!($($arg)*))
    };
}

/// The expected settings and the live settings they are compared with.
pub(crate) struct Audited<'doc> {
    /// The settings the repository should have.
    pub expected: &'doc Json,
    /// The settings the repository has.
    pub live: &'doc Json,
}

/// Who a ruleset may be bypassed by.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Actor {
    /// The identifier of the actor, when present.
    id: Option<Json>,
    /// The kind of the actor, when present.
    kind: Option<Json>,
    /// When the actor may bypass (`always` or `pull_request`), when present.
    mode: Option<Json>,
}

/// The parameter object of one rule to compare as a whole: what is expected,
/// what is live, and the member of the live object that is a list handled
/// elsewhere.
struct Parameters<'doc> {
    /// Members that are read as false when the live object omits them.
    absent_is_false: Vec<Name>,
    /// The expected parameters, when the settings pin them.
    expected: Option<&'doc Json>,
    /// A live member that is not compared here.
    ignored: Option<Name>,
    /// The label of the parameters in a message.
    label: Text,
    /// The live parameters, when the rule exists.
    live: Option<&'doc Json>,
}

/// An expected ruleset and what the repository has under that name.
struct Ruleset<'doc> {
    /// The application a status check must come from, when set.
    app_id: Option<&'doc Json>,
    /// The expected ruleset.
    expected: &'doc Json,
    /// The live ruleset, when it exists.
    live: Option<&'doc Json>,
    /// The name of the ruleset.
    name: &'doc Name,
}

/// The kinds of rule whose parameters are compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuleKind {
    /// The rule that requires pull requests.
    PullRequest,
    /// The rule that requires status checks.
    RequiredStatusChecks,
}

impl RuleKind {
    /// The rule type name the API uses.
    fn name(self) -> Text {
        Text::of(match self {
            Self::PullRequest => "pull_request",
            Self::RequiredStatusChecks => "required_status_checks",
        })
    }
}

/// A commit on the default branch, as the audit reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MergeCommit {
    /// The message to post.
    pub message: Text,
    /// The number of pull requests that are open.
    pub pulls: Count,
    /// The commit that was audited.
    pub sha: Sha,
}

/// How a value reads in a drift message.
fn shown(value: Option<&Json>) -> Text {
    match value {
        Some(found) => found.compact(),
        None => Text::of("null"),
    }
}

/// The text of a string value, or the empty text.
fn text_of(value: Option<&Json>) -> Text {
    match value.and_then(Json::as_text) {
        Some(found) => found.clone(),
        None => Text::of(""),
    }
}

/// The strings in `value`, sorted.
fn sorted_texts(value: Option<&Json>) -> Vec<Text> {
    let mut out: Vec<Text> = items_of(value)
        .iter()
        .filter_map(|item| item.as_text().cloned())
        .collect();
    out.sort();
    out
}

/// Reads a bypass actor from a ruleset entry.
fn actor(item: &Json) -> Actor {
    Actor {
        id: item.get(&Name::of("actor_id")).cloned(),
        kind: item.get(&Name::of("actor_type")).cloned(),
        mode: item.get(&Name::of("bypass_mode")).cloned(),
    }
}

/// The parameters of the live rule of `kind`.
#[must_use]
fn parameters_of(found: &Json, kind: RuleKind) -> Option<&Json> {
    items_of(found.get(&Name::of("rules")))
        .iter()
        .find(|rule| rule.text_at(&Name::of("type")) == Some(&kind.name()))
        .and_then(|rule| rule.get(&Name::of("parameters")))
}

/// The rule type names found in a live ruleset.
fn rule_types(found: &Json) -> Vec<Text> {
    let mut types: Vec<Text> = items_of(found.get(&Name::of("rules")))
        .iter()
        .filter_map(|rule| rule.text_at(&Name::of("type")).cloned())
        .collect();
    types.sort();
    types
}

/// The differences in the bypass actors of a ruleset: an actor that is not
/// expected, an expected actor that is missing, and an actor whose bypass mode
/// differs (a mode that is absent is a difference too).
fn bypass_drift(set: &Ruleset<'_>, found: &Json) -> Vec<Text> {
    let label = set.name.as_str();
    let allowed: Vec<Actor> = items_of(set.expected.get(&Name::of("bypass_actors")))
        .iter()
        .map(actor)
        .collect();
    let live: Vec<Actor> = items_of(found.get(&Name::of("bypass_actors")))
        .iter()
        .map(actor)
        .collect();
    let mut drift = Vec::new();
    for pair in &live {
        if allowed.contains(pair) {
            continue;
        }
        match allowed
            .iter()
            .find(|want| want.kind == pair.kind && want.id == pair.id)
        {
            Some(want) => drift.push(note!(
                "ruleset {label}: bypass actor {}/{} has bypass mode {}, expected {}",
                shown(pair.kind.as_ref()).as_str(),
                shown(pair.id.as_ref()).as_str(),
                shown(pair.mode.as_ref()).as_str(),
                shown(want.mode.as_ref()).as_str()
            )),
            None => drift.push(note!(
                "ruleset {label}: unexpected bypass actor {}/{} with bypass mode {}",
                shown(pair.kind.as_ref()).as_str(),
                shown(pair.id.as_ref()).as_str(),
                shown(pair.mode.as_ref()).as_str()
            )),
        }
    }
    for want in &allowed {
        if !live
            .iter()
            .any(|pair| pair.kind == want.kind && pair.id == want.id)
        {
            drift.push(note!(
                "ruleset {label}: bypass actor {}/{} with bypass mode {} is missing",
                shown(want.kind.as_ref()).as_str(),
                shown(want.id.as_ref()).as_str(),
                shown(want.mode.as_ref()).as_str()
            ));
        }
    }
    drift
}

/// The differences in the rule types of a ruleset.
fn rule_drift(set: &Ruleset<'_>, found: &Json) -> Vec<Text> {
    let label = set.name.as_str();
    let mut drift = Vec::new();
    let types = rule_types(found);
    for wanted in sorted_texts(set.expected.get(&Name::of("rules"))) {
        if !types.contains(&wanted) {
            drift.push(note!(
                "ruleset {label}: rule {} is missing",
                wanted.as_str()
            ));
        }
    }
    drift.extend(bypass_drift(set, found));
    drift
}

/// The differences between a whole expected parameter object and the live one:
/// a value that differs, a pinned parameter that is absent, and a live
/// parameter the settings do not list (a weakening the audit would otherwise
/// not look for).
fn whole_object_drift(set: &Ruleset<'_>, compared: &Parameters<'_>) -> Vec<Text> {
    let label = set.name.as_str();
    let what = compared.label.as_str();
    let Some(expected) = compared.expected else {
        return vec![note!(
            "ruleset {label}: the parameters of {what} are not pinned in settings.json"
        )];
    };
    let mut drift = Vec::new();
    for (key, want) in members_of(Some(expected)) {
        let found = compared.live.and_then(|parameters| parameters.get(key));
        let off = Json::Bool(false);
        let seen = match found {
            None if compared.absent_is_false.contains(key) => Some(&off),
            other => other,
        };
        if seen != Some(want) {
            drift.push(note!(
                "ruleset {label}: {what}.{} is not {}",
                key.as_str(),
                want.compact().as_str()
            ));
        }
    }
    for (key, got) in members_of(compared.live) {
        let listed = members_of(Some(expected))
            .iter()
            .any(|(known, _)| known == key);
        if !listed && compared.ignored.as_ref() != Some(key) {
            drift.push(note!(
                "ruleset {label}: {what} has the parameter {} = {}, which settings.json does not list",
                key.as_str(),
                got.compact().as_str()
            ));
        }
    }
    drift
}

/// The differences in the parameters of the rules of a ruleset. A rule the
/// ruleset must have is compared as a whole object, so a parameter that was
/// added or weakened shows up even when it is not one the settings name.
fn parameter_drift(set: &Ruleset<'_>, found: &Json) -> Vec<Text> {
    let label = set.name.as_str();
    let mut drift = Vec::new();
    let wanted_rules = sorted_texts(set.expected.get(&Name::of("rules")));
    if wanted_rules.contains(&RuleKind::PullRequest.name()) {
        drift.extend(whole_object_drift(
            set,
            &Parameters {
                absent_is_false: Vec::new(),
                expected: set.expected.get(&Name::of("pull_request")),
                ignored: None,
                label: RuleKind::PullRequest.name(),
                live: parameters_of(found, RuleKind::PullRequest),
            },
        ));
    }
    let status_parameters = parameters_of(found, RuleKind::RequiredStatusChecks);
    if wanted_rules.contains(&RuleKind::RequiredStatusChecks.name()) {
        drift.extend(whole_object_drift(
            set,
            &Parameters {
                absent_is_false: vec![Name::of("do_not_enforce_on_create")],
                expected: set
                    .expected
                    .get(&Name::of("required_status_checks_parameters")),
                ignored: Some(Name::of("required_status_checks")),
                label: RuleKind::RequiredStatusChecks.name(),
                live: status_parameters,
            },
        ));
    }
    let live_checks = items_of(
        status_parameters
            .and_then(|parameters| parameters.get(&Name::of("required_status_checks"))),
    );
    for check in items_of(set.expected.get(&Name::of("required_status_checks"))) {
        let context = check.get(&Name::of("context"));
        let matching = live_checks.iter().any(|candidate| {
            candidate.get(&Name::of("context")) == context
                && candidate.get(&Name::of("integration_id")) == set.app_id
        });
        if !matching || set.app_id.is_none() {
            drift.push(note!(
                "ruleset {label}: required check {} with expected source {} is missing",
                text_of(context).as_str(),
                text_of(check.get(&Name::of("app_slug"))).as_str()
            ));
        }
    }
    drift
}

/// One expected ruleset against the live one.
fn compare_ruleset(set: &Ruleset<'_>) -> Vec<Text> {
    let label = set.name.as_str();
    let Some(found) = set.live else {
        return vec![note!("ruleset {label} is missing")];
    };
    let mut drift = Vec::new();
    let target = Name::of("target");
    if found.get(&target) != set.expected.get(&target) {
        drift.push(note!("ruleset {label}: wrong target"));
    }
    let enforcement = Name::of("enforcement");
    if found.get(&enforcement) != set.expected.get(&enforcement) {
        drift.push(note!(
            "ruleset {label}: enforcement is {}",
            shown(found.get(&enforcement)).as_str()
        ));
    }
    let refs = found.dig(&keys!["conditions", "ref_name"]);
    let included = sorted_texts(refs.and_then(|found_refs| found_refs.get(&Name::of("include"))));
    let excluded = items_of(refs.and_then(|found_refs| found_refs.get(&Name::of("exclude"))));
    if included != sorted_texts(set.expected.get(&Name::of("include"))) || !excluded.is_empty() {
        drift.push(note!("ruleset {label}: ref include/exclude differ"));
    }
    drift.extend(rule_drift(set, found));
    drift.extend(parameter_drift(set, found));
    drift
}

/// The differences in one ruleset.
fn ruleset_drift(audit: &Audited<'_>) -> Vec<Text> {
    let app_id = audit.live.dig(&keys!["app", "id"]);
    let mut drift = Vec::new();
    for (name, wanted) in members_of(audit.expected.get(&Name::of("rulesets"))) {
        let found = audit
            .live
            .dig(&keys!["rulesets"])
            .and_then(|sets| sets.get(name));
        drift.extend(compare_ruleset(&Ruleset {
            app_id,
            expected: wanted,
            live: found,
            name,
        }));
    }
    drift
}

/// The differences in the repository settings.
fn repo_drift(audit: &Audited<'_>) -> Vec<Text> {
    let live_repo = audit.live.get(&Name::of("repo"));
    let mut drift = Vec::new();
    for (key, want) in members_of(audit.expected.get(&Name::of("repo"))) {
        let got = live_repo.and_then(|repo| repo.get(key));
        if got != Some(want) {
            drift.push(note!(
                "repo setting {} is {}, expected {}",
                key.as_str(),
                shown(got).as_str(),
                want.compact().as_str()
            ));
        }
    }
    let branch = Name::of("default_branch");
    if live_repo.and_then(|repo| repo.get(&branch)) != audit.expected.get(&branch) {
        drift.push(note!("default branch changed"));
    }
    drift
}

/// The differences in the Actions settings.
fn actions_drift(audit: &Audited<'_>) -> Vec<Text> {
    let mut drift = Vec::new();
    for (key, want) in members_of(audit.expected.get(&Name::of("actions"))) {
        let got = if key.as_str() == "fork_pr_approval" {
            audit.live.get(&Name::of("fork_approval"))
        } else {
            audit
                .live
                .dig(&keys!["actions"])
                .and_then(|actions| actions.get(key))
        };
        if got != Some(want) {
            drift.push(note!(
                "actions setting {} is {}, expected {}",
                key.as_str(),
                shown(got).as_str(),
                want.compact().as_str()
            ));
        }
    }
    drift
}

/// The differences in the environments.
fn environment_drift(audit: &Audited<'_>) -> Vec<Text> {
    let mut drift = Vec::new();
    for (name, wanted) in members_of(audit.expected.get(&Name::of("environments"))) {
        let got = audit
            .live
            .dig(&keys!["environments"])
            .and_then(|envs| envs.get(name));
        if got != wanted.get(&Name::of("deployment_branches")) {
            drift.push(note!(
                "environment {}: deployment branches are {}",
                name.as_str(),
                shown(got).as_str()
            ));
        }
    }
    drift
}

/// The differences in the required reviewers of the environments that must
/// have them: a publish to crates.io cannot be undone, so the `release`
/// environment must make a person approve each run.
fn reviewer_drift(audit: &Audited<'_>) -> Vec<Text> {
    let mut drift = Vec::new();
    for (name, wanted) in members_of(audit.expected.get(&Name::of("environments"))) {
        let wants = wanted
            .get(&Name::of("required_reviewers"))
            .and_then(Json::as_flag)
            .is_some_and(|flag| flag == Flag::On);
        let has = audit
            .live
            .dig(&keys!["environment_reviewers"])
            .and_then(|found| found.get(name))
            .and_then(Json::as_flag)
            .is_some_and(|flag| flag == Flag::On);
        if wants && !has {
            drift.push(note!(
                "environment {}: no required reviewers, so a run needs no approval",
                name.as_str()
            ));
        }
    }
    drift
}

/// Drift between the expected settings and the live ones.
pub(crate) fn compare(audit: &Audited<'_>) -> Vec<Text> {
    let mut drift = repo_drift(audit);
    drift.extend(ruleset_drift(audit));
    drift.extend(actions_drift(audit));
    drift.extend(environment_drift(audit));
    drift.extend(reviewer_drift(audit));
    let permissions = Name::of("permissions");
    let live_permissions = audit
        .live
        .get(&Name::of("app"))
        .and_then(|app| app.get(&permissions));
    if live_permissions != audit.expected.dig(&keys!["app", "permissions"]) {
        drift.push(note!("App permissions differ from the expected set"));
    }
    drift
}

/// Drift among recent commits: no pull request, or an automatic merge without
/// a clean verdict block.
pub(crate) fn check_merge_commits(commits: &[MergeCommit], prefix: &Text) -> Vec<Text> {
    let mut drift = Vec::new();
    for commit in commits {
        let subject = match commit.message.as_str().split_once('\n') {
            Some((first, _)) => first,
            None => commit.message.as_str(),
        };
        let short = commit.sha.short();
        if commit.pulls.get() == 0 {
            drift.push(note!(
                "commit {} reached the default branch without a pull request",
                short.as_str()
            ));
        }
        if subject.starts_with(prefix.as_str()) {
            let clean = parse_block(&commit.message).is_some_and(|block| {
                block.reasons.as_str().is_empty() && block.approver.as_str() == "none"
            });
            if !clean {
                drift.push(note!(
                    "commit {}: automatic merge lacks a clean verdict block",
                    short.as_str()
                ));
            }
        }
    }
    drift
}

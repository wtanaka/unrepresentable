//! Reading the live repository settings through the API, in the same shape as
//! `policy/settings.json`.

use crate::failure::Failure;
use crate::github::Github;
use crate::json::{items_of, members_of, Flag, Json};
use crate::types::{Name, Text};

/// The pattern of the one tag policy a release environment may have.
const TAG_PATTERN: &str = "v*";

/// One deployment policy of an environment: what it names and what kind of
/// ref it matches.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Policy {
    /// The branch or tag name or pattern.
    name: Name,
    /// `branch` or `tag`. A policy that does not say is a branch policy.
    ref_kind: Name,
}

/// The first element of `value`, or JSON null when it is absent.
fn first_or_null(value: Option<&Json>) -> Json {
    match value {
        Some(found) => found.clone(),
        None => Json::Null,
    }
}

/// Fetches the live repository settings that the audit compares.
fn repo_settings(client: &dyn Github, expected: &Json) -> Result<Json, Failure> {
    let repo = client.get(&client.repo_path(&Text::of("")))?;
    let mut wanted: Vec<Name> = members_of(expected.get(&Name::of("repo")))
        .iter()
        .map(|(key, _)| key.clone())
        .collect();
    wanted.push(Name::of("default_branch"));
    let mut settings = Vec::new();
    for key in wanted {
        let value = first_or_null(repo.get(&key));
        settings.push((key, value));
    }
    Ok(Json::object_of(settings))
}

/// Fetches the live rulesets with their details.
fn rulesets(client: &dyn Github) -> Result<Json, Failure> {
    let summaries = client
        .get(&client.repo_path(&Text::of("/rulesets?includes_parents=false&per_page=100")))?;
    let mut details = Vec::new();
    for summary in items_of(Some(&summaries)) {
        let (Some(name), Some(id)) = (
            summary.text_at(&Name::of("name")),
            summary.count_at(&Name::of("id")),
        ) else {
            continue;
        };
        let detail =
            client.get(&client.repo_path(&Text::from_string(format!("/rulesets/{}", id.get()))))?;
        details.push((Name::of(name.as_str()), detail));
    }
    Ok(Json::object_of(details))
}

/// The environment as the API reports it, or none when it is missing.
#[must_use]
fn environment_of(client: &dyn Github, name: &Name) -> Option<Json> {
    let Ok(found) = client.get(&client.repo_path(&Text::from_string(format!(
        "/environments/{}",
        name.as_str()
    )))) else {
        return None;
    };
    Some(found)
}

/// How an environment restricts deployments, in `settings.json` words.
/// `default-branch-only` means one custom policy that names exactly the
/// default branch; `protected_branches` is not that, because it admits every
/// branch that has any protection rule. `tags-only` means one tag policy with
/// the pattern `v*`.
pub(super) fn environment_branches(
    client: &dyn Github,
    name: &Name,
    default_branch: &Text,
) -> Result<Text, Failure> {
    let Some(env) = environment_of(client, name) else {
        return Ok(Text::of("missing"));
    };
    let policy = env.get(&Name::of("deployment_branch_policy"));
    let flag = |key: &str| {
        policy
            .and_then(|found| found.get(&Name::of(key)))
            .and_then(Json::as_flag)
            .is_some_and(|value| value == Flag::On)
    };
    if flag("protected_branches") {
        return Ok(Text::of("protected-branches"));
    }
    if !flag("custom_branch_policies") {
        return Ok(Text::of("any-branch"));
    }
    let listed = client.get(&client.repo_path(&Text::from_string(format!(
        "/environments/{}/deployment-branch-policies",
        name.as_str()
    ))))?;
    let mut policies: Vec<Policy> = items_of(listed.get(&Name::of("branch_policies")))
        .iter()
        .filter_map(|item| {
            let found = item.text_at(&Name::of("name"))?;
            let kind = match item.text_at(&Name::of("type")) {
                Some(given) => Name::of(given.as_str()),
                None => Name::of("branch"),
            };
            Some(Policy {
                name: Name::of(found.as_str()),
                ref_kind: kind,
            })
        })
        .collect();
    policies.sort();
    let only = |kind: &str, wanted: &str| {
        policies
            == [Policy {
                name: Name::of(wanted),
                ref_kind: Name::of(kind),
            }]
    };
    Ok(Text::from_string(
        if only("branch", default_branch.as_str()) {
            "default-branch-only".to_owned()
        } else if only("tag", TAG_PATTERN) {
            "tags-only".to_owned()
        } else {
            let shown: Vec<String> = policies
                .iter()
                .map(|item| format!("{}:{}", item.ref_kind.as_str(), item.name.as_str()))
                .collect();
            format!("custom:{}", shown.join(","))
        },
    ))
}

/// Whether each environment the expected settings name requires reviewers:
/// a protection rule of type `required_reviewers` that names at least one.
pub(super) fn environment_reviewers(client: &dyn Github, expected: &Json) -> Json {
    let mut found = Vec::new();
    for (name, _) in members_of(expected.get(&Name::of("environments"))) {
        let has = environment_of(client, name).is_some_and(|env| {
            items_of(env.get(&Name::of("protection_rules")))
                .iter()
                .any(|rule| {
                    rule.text_at(&Name::of("type")).map(Text::as_str) == Some("required_reviewers")
                        && !items_of(rule.get(&Name::of("reviewers"))).is_empty()
                })
        });
        let flag = if has { Flag::On } else { Flag::Off };
        found.push((name.clone(), Json::flag_of(flag)));
    }
    Json::object_of(found)
}

/// Fetches the live environments that the expected settings name.
fn environments(client: &dyn Github, expected: &Json) -> Result<Json, Failure> {
    let default_branch = match expected
        .get(&Name::of("default_branch"))
        .and_then(Json::as_text)
    {
        Some(found) => found.clone(),
        None => Text::of(""),
    };
    let mut found = Vec::new();
    for (name, _) in members_of(expected.get(&Name::of("environments"))) {
        let branches = environment_branches(client, name, &default_branch)?;
        found.push((name.clone(), Json::text_of(&branches)));
    }
    Ok(Json::object_of(found))
}

/// The live settings: `repo`, `rulesets`, `actions`, `fork_approval`,
/// `environments` and `app`.
pub(crate) fn gather(client: &dyn Github, expected: &Json) -> Result<Json, Failure> {
    let actions = client.get(&client.repo_path(&Text::of("/actions/permissions/workflow")))?;
    let fork = client.get(&client.repo_path(&Text::of(
        "/actions/permissions/fork-pr-contributor-approval",
    )))?;
    let slug = match expected.dig(&keys!["app", "slug"]).and_then(Json::as_text) {
        Some(found) => found.clone(),
        None => Text::of(""),
    };
    let app = client.get(&Text::from_string(format!("/apps/{}", slug.as_str())))?;
    Ok(Json::object_of(vec![
        (Name::of("repo"), repo_settings(client, expected)?),
        (Name::of("rulesets"), rulesets(client)?),
        (Name::of("actions"), actions),
        (
            Name::of("fork_approval"),
            first_or_null(fork.get(&Name::of("approval_policy"))),
        ),
        (Name::of("environments"), environments(client, expected)?),
        (
            Name::of("environment_reviewers"),
            environment_reviewers(client, expected),
        ),
        (
            Name::of("app"),
            Json::object_of(vec![
                (Name::of("id"), first_or_null(app.get(&Name::of("id")))),
                (
                    Name::of("permissions"),
                    first_or_null(app.get(&Name::of("permissions"))),
                ),
            ]),
        ),
    ]))
}

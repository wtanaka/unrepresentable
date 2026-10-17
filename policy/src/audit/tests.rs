use super::live::{environment_branches, environment_reviewers};
use super::{check_merge_commits, compare, Audited, MergeCommit};
use crate::json::{Flag, Json};
use crate::reason::{Code, Commits};
use crate::testkit::{json_of, Fail, FakeGithub};
use crate::types::{Count, Message, Name, Sha, Text};
use crate::verdict::{merge_message, BlockFacts, PullNumber};

macro_rules! drift_of {
    ($expected:expr_2021, $live:expr_2021) => {
        compare(&Audited {
            expected: &$expected,
            live: &$live,
        })
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect::<Vec<String>>()
        .join(" | ")
    };
}

macro_rules! commit {
    ($message:expr_2021, $pulls:literal) => {
        MergeCommit {
            sha: sample_sha(),
            message: Text::of($message),
            pulls: Count::new($pulls),
        }
    };
}

/// Live rules made of two parameter texts.
macro_rules! live_rules {
    ($pull_request:expr_2021, $status_checks:expr_2021) => {
        LiveRules {
            pull_request: Text::from_string($pull_request),
            status_checks: Text::from_string($status_checks),
        }
    };
}

/// An environment made of its settings and its policies.
macro_rules! environment {
    ($settings:expr_2021, $policies:expr_2021) => {
        Environment {
            policies: Text::of($policies),
            settings: Text::of($settings),
        }
    };
}

const SETTINGS: &str = include_str!("../../../policy/settings.json");
const CUSTOM_POLICIES: &str =
    "{\"deployment_branch_policy\":{\"protected_branches\":false,\"custom_branch_policies\":true}}";
const PINNED_PULL_REQUEST: &str = "{\"allowed_merge_methods\":[\"squash\"],\"dismiss_stale_reviews_on_push\":true,\"require_code_owner_review\":false,\"require_last_push_approval\":false,\"required_approving_review_count\":0,\"required_review_thread_resolution\":false}";
const PINNED_STATUS_CHECKS: &str = "{\"strict_required_status_checks_policy\":true,\"required_status_checks\":[{\"context\":\"policy\",\"integration_id\":9}]}";

/// The live bypass actors the pinned settings expect on `default-branch`.
const PINNED_BYPASS: &str =
    "[{\"actor_type\":\"RepositoryRole\",\"actor_id\":5,\"bypass_mode\":\"pull_request\"}]";

/// The parameters the live pull request and status check rules have.
struct LiveRules {
    /// The parameters of the pull request rule.
    pull_request: Text,
    /// The parameters of the required status checks rule.
    status_checks: Text,
}

/// What the fake API says of an environment named `policy`.
struct Environment {
    /// The deployment branch policies listed for it.
    policies: Text,
    /// The environment itself.
    settings: Text,
}

#[test]
fn audit_compare_detects_drift() {
    assert_eq!(
        check_audit_compare_detects_drift(),
        Ok(()),
        "audit_compare_detects_drift"
    );
}

fn check_audit_compare_detects_drift() -> Result<(), Fail> {
    let expected = json_of(&Text::of(SETTINGS))?;
    let live = json_of(&Text::of(
        "{\"repo\":{\"allow_auto_merge\":true,\"allow_squash_merge\":true,\"allow_merge_commit\":false,\"allow_rebase_merge\":false,\"default_branch\":\"main\"},\"rulesets\":{},\"actions\":{\"default_workflow_permissions\":\"write\",\"can_approve_pull_request_reviews\":true},\"fork_approval\":\"first_time_contributors\",\"environments\":{},\"app\":{\"id\":1,\"permissions\":{\"checks\":\"write\"}}}",
    ))?;
    let drift = drift_of!(expected, live);
    for needle in [
        "allow_auto_merge",
        "default_workflow_permissions",
        "can_approve_pull_request_reviews",
        "fork_pr_approval",
        "ruleset default-branch is missing",
        "App permissions",
    ] {
        ensure!(drift.contains(needle), "{needle} is reported in: {drift}");
    }
    Ok(())
}

#[test]
fn matching_settings_have_no_drift() {
    assert_eq!(
        check_matching_settings_have_no_drift(),
        Ok(()),
        "matching_settings_have_no_drift"
    );
}

fn check_matching_settings_have_no_drift() -> Result<(), Fail> {
    let expected = json_of(&Text::of(
        "{\"default_branch\":\"main\",\"repo\":{\"a\":true},\"rulesets\":{},\"actions\":{\"fork_pr_approval\":\"all\"},\"environments\":{\"policy\":{\"deployment_branches\":\"default-branch-only\"}},\"app\":{\"permissions\":{\"checks\":\"write\"}}}",
    ))?;
    let live = json_of(&Text::of(
        "{\"repo\":{\"a\":true,\"default_branch\":\"main\"},\"rulesets\":{},\"actions\":{},\"fork_approval\":\"all\",\"environments\":{\"policy\":\"default-branch-only\"},\"app\":{\"id\":1,\"permissions\":{\"checks\":\"write\"}}}",
    ))?;
    ensure_eq!(
        drift_of!(expected, live),
        String::new(),
        "identical settings agree"
    );
    Ok(())
}

#[test]
fn ruleset_drift_is_specific() {
    assert_eq!(
        check_ruleset_drift_is_specific(),
        Ok(()),
        "ruleset_drift_is_specific"
    );
}

fn check_ruleset_drift_is_specific() -> Result<(), Fail> {
    let expected = json_of(&Text::of(
        "{\"default_branch\":\"main\",\"repo\":{},\"actions\":{},\"environments\":{},\"app\":{\"permissions\":{}},\"rulesets\":{\"rs\":{\"target\":\"branch\",\"enforcement\":\"active\",\"include\":[\"~DEFAULT_BRANCH\"],\"rules\":[\"pull_request\",\"deletion\"],\"pull_request\":{\"require_last_push_approval\":true},\"required_status_checks\":[{\"context\":\"policy\",\"app_slug\":\"unrepresentable-policy\"}],\"bypass_actors\":[]}}}",
    ))?;
    let live = json_of(&Text::of(
        "{\"repo\":{\"default_branch\":\"main\"},\"actions\":{},\"environments\":{},\"app\":{\"id\":9,\"permissions\":{}},\"rulesets\":{\"rs\":{\"target\":\"branch\",\"enforcement\":\"disabled\",\"conditions\":{\"ref_name\":{\"include\":[\"~DEFAULT_BRANCH\"],\"exclude\":[]}},\"rules\":[{\"type\":\"pull_request\",\"parameters\":{\"require_last_push_approval\":false}},{\"type\":\"required_status_checks\",\"parameters\":{\"required_status_checks\":[{\"context\":\"policy\",\"integration_id\":1}]}}],\"bypass_actors\":[{\"actor_type\":\"Team\",\"actor_id\":3}]}}}",
    ))?;
    let drift = drift_of!(expected, live);
    for needle in [
        "enforcement is \"disabled\"",
        "rule deletion is missing",
        "unexpected bypass actor \"Team\"/3 with bypass mode null",
        "pull_request.require_last_push_approval is not true",
        "required check policy with expected source unrepresentable-policy is missing",
    ] {
        ensure!(drift.contains(needle), "{needle} is reported in: {drift}");
    }
    Ok(())
}

fn sample_sha() -> Sha {
    sha!('a')
}

#[test]
fn audit_merge_commit_blocks() {
    assert_eq!(
        check_audit_merge_commit_blocks(),
        Ok(()),
        "audit_merge_commit_blocks"
    );
}

fn check_audit_merge_commit_blocks() -> Result<(), Fail> {
    let prefix = Text::of("Squash merge of pull request #");
    let facts = BlockFacts {
        approvers: Vec::new(),
        codes: Vec::<Code>::new(),
        commits: Commits {
            base: sha!('b'),
            head: sample_sha(),
        },
        policy_commit: sha!('c'),
    };
    let Some(number) = PullNumber::parse(&Text::of("3")) else {
        return Err(Fail::new(&Message::of("3 is a pull request number")));
    };
    let good = merge_message(number, &facts)?;
    let message = format!("{}\n\n{}", good.subject.as_str(), good.body.as_str());
    ensure_eq!(
        check_merge_commits(&[commit!(&message, 1)], &prefix).len(),
        0,
        "a clean automatic merge is fine"
    );
    ensure_eq!(
        check_merge_commits(
            &[commit!("Squash merge of pull request #3\n\nno block", 1)],
            &prefix
        )
        .len(),
        1,
        "an automatic merge without a block drifts"
    );
    ensure_eq!(
        check_merge_commits(&[commit!("Direct push", 0)], &prefix).len(),
        1,
        "a commit without a pull request drifts"
    );
    Ok(())
}

/// The drift of the `default-branch` ruleset when the live rules have the given
/// parameters.
fn parameters_drift(rules: &LiveRules) -> Result<Text, Fail> {
    ruleset_drift_with(rules, &Text::of(PINNED_BYPASS))
}

/// The drift of the `default-branch` ruleset when the live rules have the given
/// parameters and the live ruleset has the given bypass actors (a JSON list).
fn ruleset_drift_with(rules: &LiveRules, actors: &Text) -> Result<Text, Fail> {
    let expected = json_of(&Text::of(SETTINGS))?;
    let live = json_of(&Text::from_string(format!(
        "{{\"repo\":{{\"default_branch\":\"main\"}},\"actions\":{{}},\"environments\":{{}},\"app\":{{\"id\":9,\"permissions\":{{}}}},\"rulesets\":{{\"default-branch\":{{\"target\":\"branch\",\"enforcement\":\"active\",\"conditions\":{{\"ref_name\":{{\"include\":[\"~DEFAULT_BRANCH\"],\"exclude\":[]}}}},\"rules\":[{{\"type\":\"pull_request\",\"parameters\":{}}},{{\"type\":\"required_status_checks\",\"parameters\":{}}},{{\"type\":\"non_fast_forward\"}},{{\"type\":\"deletion\"}},{{\"type\":\"required_linear_history\"}}],\"bypass_actors\":{}}}}}}}",
        rules.pull_request.as_str(),
        rules.status_checks.as_str(),
        actors.as_str()
    )))?;
    Ok(Text::from_string(
        drift_of!(expected, live)
            .split(" | ")
            .filter(|line| line.starts_with("ruleset default-branch:"))
            .collect::<Vec<&str>>()
            .join(" | "),
    ))
}

#[test]
fn the_whole_rule_parameter_object_is_compared() {
    assert_eq!(
        check_the_whole_rule_parameter_object_is_compared(),
        Ok(()),
        "the_whole_rule_parameter_object_is_compared"
    );
}

/// Live rules that each weaken one parameter, with what the audit must say.
fn weakened_cases() -> Vec<(LiveRules, Text)> {
    let stale = "\"dismiss_stale_reviews_on_push\":true";
    let count = "\"required_approving_review_count\":0";
    let owner = "\"require_code_owner_review\":false,";
    let strict = "\"strict_required_status_checks_policy\":true";
    let pinned = || {
        (
            PINNED_PULL_REQUEST.to_owned(),
            PINNED_STATUS_CHECKS.to_owned(),
        )
    };
    vec![
        (
            live_rules!(
                PINNED_PULL_REQUEST.replace(stale, "\"dismiss_stale_reviews_on_push\":false"),
                pinned().1
            ),
            Text::of("pull_request.dismiss_stale_reviews_on_push is not true"),
        ),
        (
            live_rules!(
                PINNED_PULL_REQUEST.replace(count, "\"required_approving_review_count\":1"),
                pinned().1
            ),
            Text::of("pull_request.required_approving_review_count is not 0"),
        ),
        (
            live_rules!(
                PINNED_PULL_REQUEST.replace('}', ",\"a_new_exemption\":true}"),
                pinned().1
            ),
            Text::of("pull_request has the parameter a_new_exemption = true, which settings.json does not list"),
        ),
        (
            live_rules!(PINNED_PULL_REQUEST.replace(owner, ""), pinned().1),
            Text::of("pull_request.require_code_owner_review is not false"),
        ),
        (
            live_rules!(
                pinned().0,
                PINNED_STATUS_CHECKS.replace(strict, "\"strict_required_status_checks_policy\":false")
            ),
            Text::of("required_status_checks.strict_required_status_checks_policy is not true"),
        ),
        (
            live_rules!(
                pinned().0,
                PINNED_STATUS_CHECKS.replace(
                    strict,
                    "\"do_not_enforce_on_create\":true,\"strict_required_status_checks_policy\":true"
                )
            ),
            Text::of("required_status_checks.do_not_enforce_on_create is not false"),
        ),
    ]
}

fn check_the_whole_rule_parameter_object_is_compared() -> Result<(), Fail> {
    ensure_eq!(
        parameters_drift(&live_rules!(
            PINNED_PULL_REQUEST.to_owned(),
            PINNED_STATUS_CHECKS.to_owned()
        ))?,
        Text::of(""),
        "the pinned parameters are no drift"
    );
    for (rules, needle) in weakened_cases() {
        let drift = parameters_drift(&rules)?;
        ensure!(
            drift.as_str().contains(needle.as_str()),
            "{} is reported in: {}",
            needle.as_str(),
            drift.as_str()
        );
    }
    Ok(())
}

/// The fake API for an environment named `policy`.
fn fake_of(env: &Environment) -> Result<FakeGithub, Fail> {
    Ok(FakeGithub::new()
        .route(&Text::of("/environments/policy"), &json_of(&env.settings)?)
        .route(
            &Text::of("/environments/policy/deployment-branch-policies"),
            &json_of(&env.policies)?,
        ))
}

/// What the audit reads of an environment that has these deployment policies.
fn kind_of(env: &Environment) -> Result<Text, Fail> {
    Ok(environment_branches(
        &fake_of(env)?,
        &Name::of("policy"),
        &Text::of("main"),
    )?)
}

#[test]
fn environment_deployment_policies_must_be_exact() {
    assert_eq!(
        check_environment_deployment_policies_must_be_exact(),
        Ok(()),
        "environment_deployment_policies_must_be_exact"
    );
}

fn check_environment_deployment_policies_must_be_exact() -> Result<(), Fail> {
    ensure_eq!(
        kind_of(&environment!(
            "{\"deployment_branch_policy\":{\"protected_branches\":true,\"custom_branch_policies\":false}}",
            "{}"
        ))?,
        Text::of("protected-branches"),
        "protected branches admit every protected branch, not only the default one"
    );
    for (policies, kind, why) in [
        (
            "{\"branch_policies\":[{\"name\":\"main\",\"type\":\"branch\"}]}",
            "default-branch-only",
            "one branch policy naming the default branch",
        ),
        (
            "{\"branch_policies\":[{\"name\":\"main\"}]}",
            "default-branch-only",
            "a policy without a type is a branch policy",
        ),
        (
            "{\"branch_policies\":[{\"name\":\"v*\",\"type\":\"tag\"}]}",
            "tags-only",
            "one tag policy with the pattern v*",
        ),
    ] {
        ensure_eq!(
            kind_of(&environment!(CUSTOM_POLICIES, policies))?,
            Text::of(kind),
            "{why}"
        );
    }
    ensure_eq!(
        kind_of(&environment!("{\"deployment_branch_policy\":null}", "{}"))?,
        Text::of("any-branch"),
        "no restriction at all"
    );
    Ok(())
}

#[test]
fn near_misses_are_neither_default_branch_only_nor_tags_only() {
    assert_eq!(
        check_near_misses_are_neither_default_branch_only_nor_tags_only(),
        Ok(()),
        "near_misses_are_neither_default_branch_only_nor_tags_only"
    );
}

fn check_near_misses_are_neither_default_branch_only_nor_tags_only() -> Result<(), Fail> {
    for (policies, why) in [
        ("{\"branch_policies\":[{\"name\":\"main\",\"type\":\"tag\"}]}", "a tag named main"),
        ("{\"branch_policies\":[{\"name\":\"main\",\"type\":\"branch\"},{\"name\":\"dev\",\"type\":\"branch\"}]}", "another branch next to main"),
        ("{\"branch_policies\":[{\"name\":\"ma*\",\"type\":\"branch\"}]}", "a pattern for the default branch"),
        ("{\"branch_policies\":[]}", "no policy"),
        ("{\"branch_policies\":[{\"name\":\"v*\",\"type\":\"branch\"}]}", "a branch named v*"),
        ("{\"branch_policies\":[{\"name\":\"v1\",\"type\":\"tag\"}]}", "one tag, not the pattern"),
        ("{\"branch_policies\":[{\"name\":\"v*\",\"type\":\"tag\"},{\"name\":\"v-evil\",\"type\":\"branch\"}]}", "a branch policy next to the tag policy"),
        ("{\"branch_policies\":[{\"name\":\"vx\",\"type\":\"tag\"},{\"name\":\"v*\",\"type\":\"tag\"}]}", "two tag policies"),
    ] {
        let kind = kind_of(&environment!(CUSTOM_POLICIES, policies))?;
        ensure!(
            kind.as_str() != "default-branch-only" && kind.as_str() != "tags-only",
            "{why} is {}",
            kind.as_str()
        );
    }
    Ok(())
}

#[test]
fn the_release_environment_must_have_required_reviewers() {
    assert_eq!(
        check_the_release_environment_must_have_required_reviewers(),
        Ok(()),
        "the_release_environment_must_have_required_reviewers"
    );
}

/// `true` or `false`: whether the audit finds a reviewer rule in an
/// environment with these protection rules.
fn reviewers_flag(rules: &Text) -> Result<Text, Fail> {
    let env = Environment {
        policies: Text::of("{}"),
        settings: Text::from_string(format!("{{\"protection_rules\":{}}}", rules.as_str())),
    };
    let wants = json_of(&Text::of("{\"environments\":{\"policy\":{}}}"))?;
    environment_reviewers(&fake_of(&env)?, &wants)
        .dig(&keys!["policy"])
        .map(Json::compact)
        .ok_or_else(|| Fail::new(&Message::of("no entry")))
}

/// The drift of a `release` environment that the live side says has (or has
/// not) required reviewers.
fn release_drift(reviewers: Flag) -> Result<Text, Fail> {
    let live = json_of(&Text::from_string(format!(
        "{{\"repo\":{{\"default_branch\":\"main\"}},\"actions\":{{}},\"rulesets\":{{}},\"environments\":{{\"release\":\"tags-only\"}},\"environment_reviewers\":{{\"release\":{}}},\"app\":{{\"id\":1,\"permissions\":{{}}}}}}",
        Json::flag_of(reviewers).compact().as_str()
    )))?;
    let settings = json_of(&Text::of("{\"default_branch\":\"main\",\"repo\":{},\"rulesets\":{},\"actions\":{},\"environments\":{\"release\":{\"deployment_branches\":\"tags-only\",\"required_reviewers\":true}},\"app\":{\"permissions\":{}}}"))?;
    Ok(Text::from_string(drift_of!(settings, live)))
}

fn check_the_release_environment_must_have_required_reviewers() -> Result<(), Fail> {
    let expected = json_of(&Text::of(SETTINGS))?;
    ensure!(
        expected
            .dig(&keys!["environments", "release", "required_reviewers"])
            .is_some(),
        "settings.json requires reviewers on the release environment"
    );
    ensure_eq!(
        reviewers_flag(&Text::of(
            "[{\"type\":\"required_reviewers\",\"reviewers\":[{\"type\":\"User\"}]}]"
        ))?,
        Text::of("true"),
        "a reviewer is required"
    );
    ensure_eq!(
        reviewers_flag(&Text::of("[{\"type\":\"wait_timer\"}]"))?,
        Text::of("false"),
        "no reviewer rule"
    );
    ensure_eq!(
        reviewers_flag(&Text::of(
            "[{\"type\":\"required_reviewers\",\"reviewers\":[]}]"
        ))?,
        Text::of("false"),
        "a reviewer rule that names nobody"
    );
    ensure!(
        release_drift(Flag::Off)?
            .as_str()
            .contains("environment release: no required reviewers"),
        "an unguarded release environment is drift"
    );
    ensure_eq!(
        release_drift(Flag::On)?,
        Text::of(""),
        "a guarded release environment is not"
    );
    Ok(())
}

#[test]
fn the_pinned_settings_do_not_use_the_native_last_push_rule_and_let_the_owner_bypass_tags() {
    assert_eq!(
        check_the_pinned_settings_do_not_use_the_native_last_push_rule_and_let_the_owner_bypass_tags(),
        Ok(()),
        "the_pinned_settings_do_not_use_the_native_last_push_rule_and_let_the_owner_bypass_tags"
    );
}

fn check_the_pinned_settings_do_not_use_the_native_last_push_rule_and_let_the_owner_bypass_tags(
) -> Result<(), Fail> {
    let expected = json_of(&Text::of(SETTINGS))?;
    let pull = expected
        .dig(&keys!["rulesets", "default-branch", "pull_request"])
        .map(Json::compact);
    ensure!(
        pull.as_ref().is_some_and(|text| text
            .as_str()
            .contains("\"require_last_push_approval\":false")
            && text.as_str().contains("\"dismiss_stale_reviews_on_push\":true")
            && text
                .as_str()
                .contains("\"required_approving_review_count\":0")),
        "no native last-push approval (it can block the App's merge at zero required reviews); stale approvals are still dismissed: {pull:?}"
    );
    for name in ["release-tags", "branch-creation"] {
        let actors = expected
            .dig(&[
                Name::of("rulesets"),
                Name::of(name),
                Name::of("bypass_actors"),
            ])
            .map(Json::compact);
        ensure_eq!(
            actors,
            Some(Text::of(
                "[{\"actor_id\":2,\"actor_type\":\"RepositoryRole\",\"bypass_mode\":\"always\"},{\"actor_id\":5,\"actor_type\":\"RepositoryRole\",\"bypass_mode\":\"always\"}]"
            )),
            "{name} can be bypassed by Maintain and by Admin (the owner of a personal repository is an admin)"
        );
    }
    ensure_eq!(
        expected
            .dig(&keys!["rulesets", "default-branch", "bypass_actors"])
            .map(Json::compact),
        Some(Text::of(
            "[{\"actor_id\":5,\"actor_type\":\"RepositoryRole\",\"bypass_mode\":\"pull_request\"}]"
        )),
        "only the admin role bypasses the default branch ruleset, and only through a pull request"
    );
    Ok(())
}

#[test]
fn the_pinned_bypass_actors_are_not_drift() {
    assert_eq!(
        check_the_pinned_bypass_actors_are_not_drift(),
        Ok(()),
        "the_pinned_bypass_actors_are_not_drift"
    );
}

fn check_the_pinned_bypass_actors_are_not_drift() -> Result<(), Fail> {
    let drift = ruleset_drift_with(&pinned_rules(), &Text::of(PINNED_BYPASS))?;
    ensure_eq!(
        drift,
        Text::of(""),
        "the pinned actor and mode are not drift"
    );
    Ok(())
}

/// The pinned parameters of the pull request rule and the status check rule.
fn pinned_rules() -> LiveRules {
    live_rules!(
        PINNED_PULL_REQUEST.to_owned(),
        PINNED_STATUS_CHECKS.to_owned()
    )
}

#[test]
fn a_live_always_mode_on_the_default_branch_is_drift() {
    assert_eq!(
        check_a_live_always_mode_on_the_default_branch_is_drift(),
        Ok(()),
        "a_live_always_mode_on_the_default_branch_is_drift"
    );
}

fn check_a_live_always_mode_on_the_default_branch_is_drift() -> Result<(), Fail> {
    let drift = ruleset_drift_with(
        &pinned_rules(),
        &Text::of(
            "[{\"actor_type\":\"RepositoryRole\",\"actor_id\":5,\"bypass_mode\":\"always\"}]",
        ),
    )?;
    ensure_eq!(
        drift,
        Text::of(
            "ruleset default-branch: bypass actor \"RepositoryRole\"/5 has bypass mode \"always\", expected \"pull_request\""
        ),
        "a bypass that also allows a direct push is reported"
    );
    let absent = ruleset_drift_with(
        &pinned_rules(),
        &Text::of("[{\"actor_type\":\"RepositoryRole\",\"actor_id\":5}]"),
    )?;
    ensure!(
        absent
            .as_str()
            .contains("has bypass mode null, expected \"pull_request\""),
        "a live actor without a mode is reported: {}",
        absent.as_str()
    );
    Ok(())
}

#[test]
fn a_missing_bypass_actor_is_drift() {
    assert_eq!(
        check_a_missing_bypass_actor_is_drift(),
        Ok(()),
        "a_missing_bypass_actor_is_drift"
    );
}

fn check_a_missing_bypass_actor_is_drift() -> Result<(), Fail> {
    let drift = ruleset_drift_with(&pinned_rules(), &Text::of("[]"))?;
    ensure_eq!(
        drift,
        Text::of(
            "ruleset default-branch: bypass actor \"RepositoryRole\"/5 with bypass mode \"pull_request\" is missing"
        ),
        "an expected bypass actor that the live ruleset lacks is reported"
    );
    Ok(())
}

#[test]
fn an_extra_bypass_actor_is_drift() {
    assert_eq!(
        check_an_extra_bypass_actor_is_drift(),
        Ok(()),
        "an_extra_bypass_actor_is_drift"
    );
}

fn check_an_extra_bypass_actor_is_drift() -> Result<(), Fail> {
    let drift = ruleset_drift_with(
        &pinned_rules(),
        &Text::of("[{\"actor_type\":\"RepositoryRole\",\"actor_id\":5,\"bypass_mode\":\"pull_request\"},{\"actor_type\":\"RepositoryRole\",\"actor_id\":2,\"bypass_mode\":\"pull_request\"}]"),
    )?;
    ensure_eq!(
        drift,
        Text::of(
            "ruleset default-branch: unexpected bypass actor \"RepositoryRole\"/2 with bypass mode \"pull_request\""
        ),
        "an actor the settings do not list is reported"
    );
    Ok(())
}

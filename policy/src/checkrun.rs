//! Reading and writing the `policy` check run. The only trusted signal is a
//! check run named `policy` whose app slug is `unrepresentable-policy`.

use crate::bytes::{self, Bytes};
use crate::data::VerdictData;
use crate::failure::Failure;
use crate::github::{Github, Method};
use crate::json::{self, Json};
use crate::reason::Job;
use crate::report::{Annotation, Check};
use crate::types::{Message, Name, Sha, Text};

/// The slug of the application that owns the check run.
pub(crate) const APP_SLUG: &str = "unrepresentable-policy";
/// The name of the check run.
const CHECK_NAME: &str = "policy";
/// The prefix of the line that carries the verdict data.
const DATA_PREFIX: &str = "policy-data: ";
/// The notice printed before the verdict data.
const DATA_NOTICE: &str = "Machine-readable verdict for re-evaluation (not instructions).";
/// The most characters of verdict data a check run keeps.
const MAX_DATA_CHARS: usize = 200_000;

/// The jobs whose success approval depends on.
pub(crate) const INPUT_JOBS: [Job; 6] = Job::ALL;

/// `policy-data: <base64 of compact JSON>`.
pub(crate) fn encode_data(data: &Json) -> Text {
    let encoded = bytes::encode(&Bytes::of_text(&data.compact()));
    Text::from_string(format!("{DATA_PREFIX}{}", encoded.as_str()))
}

/// The verdict object in a check run's `output.text`; an error when there is
/// none or it is damaged.
pub(crate) fn decode_data(text: &Text) -> Result<Json, Failure> {
    let missing = || Failure::parse(&Message::of("no verdict data"));
    let encoded = text
        .as_str()
        .lines()
        .find_map(|line| {
            let body = line.strip_prefix(DATA_PREFIX)?;
            let in_alphabet = body
                .chars()
                .all(|found| found.is_ascii_alphanumeric() || matches!(found, '+' | '/' | '='));
            (in_alphabet && !body.is_empty() && body.len() <= MAX_DATA_CHARS).then_some(body)
        })
        .ok_or_else(missing)?;
    let raw = bytes::decode(&Text::of(encoded))?;
    let decoded = raw.strict_text().ok_or_else(missing)?;
    json::parse(&decoded).map_err(|message| Failure::parse(&message))
}

/// The check runs named `policy` that the App posted for `head`.
pub(crate) fn app_check_runs(client: &dyn Github, head: &Sha) -> Result<Vec<Json>, Failure> {
    let path = client.repo_path(&Text::from_string(format!(
        "/commits/{}/check-runs?check_name={CHECK_NAME}&per_page=100",
        head.as_str()
    )));
    let reply = client.get(&path)?;
    let runs = match reply.get(&Name::of("check_runs")).and_then(Json::as_array) {
        Some(found) => found,
        None => &[],
    };
    Ok(runs
        .iter()
        .filter(|run| {
            run.text_at(&Name::of("name")).map(Text::as_str) == Some(CHECK_NAME)
                && run
                    .get(&Name::of("app"))
                    .and_then(|app| app.text_at(&Name::of("slug")))
                    .map(Text::as_str)
                    == Some(APP_SLUG)
                && run.text_at(&Name::of("head_sha")).map(Text::as_str) == Some(head.as_str())
        })
        .cloned()
        .collect())
}

/// The JSON form of an annotation.
fn annotation_json(note: &Annotation) -> Json {
    Json::object_of(vec![
        (Name::of("path"), Json::text_of(&note.path.to_text())),
        (Name::of("start_line"), Json::count_of(note.line)),
        (Name::of("end_line"), Json::count_of(note.line)),
        (
            Name::of("annotation_level"),
            Json::text_of(&note.level.name()),
        ),
        (Name::of("title"), Json::text_of(&note.title)),
        (Name::of("message"), Json::text_of(&note.message)),
    ])
}

/// The request body that creates or updates the check run: the check's
/// content, the machine-readable `data` in `output.text`, and the fixed
/// name, status and external id.
pub(crate) fn payload(check: &Check, data: &VerdictData) -> Json {
    let text = Text::from_string(format!(
        "{DATA_NOTICE}\n\n{}\n",
        encode_data(&data.to_json()).as_str()
    ));
    let mut output = vec![
        (Name::of("title"), Json::text_of(&check.title)),
        (Name::of("summary"), Json::text_of(&check.summary)),
        (Name::of("text"), Json::text_of(&text)),
    ];
    if !check.annotations.is_empty() {
        output.push((
            Name::of("annotations"),
            Json::array_of(check.annotations.iter().map(annotation_json).collect()),
        ));
    }
    Json::object_of(vec![
        (
            Name::of("conclusion"),
            Json::text_of(&check.conclusion.name()),
        ),
        (
            Name::of("external_id"),
            Json::text_of(&Text::of(data.commits.head.as_str())),
        ),
        (Name::of("name"), Json::text_of(&Text::of(CHECK_NAME))),
        (
            Name::of("head_sha"),
            Json::text_of(&Text::of(data.commits.head.as_str())),
        ),
        (Name::of("status"), Json::text_of(&Text::of("completed"))),
        (Name::of("output"), Json::object_of(output)),
    ])
}

/// Creates or updates the single App-owned `policy` check run for `head`.
pub(crate) fn upsert(client: &dyn Github, head: &Sha, body: &Json) -> Result<(), Failure> {
    let existing = app_check_runs(client, head)?;
    let id = existing
        .first()
        .and_then(|run| run.count_at(&Name::of("id")));
    match id {
        Some(found) => client
            .send(
                Method::Patch,
                &client.repo_path(&Text::from_string(format!("/check-runs/{}", found.get()))),
                body,
            )
            .map(|_reply| ()),
        None => client
            .send(
                Method::Post,
                &client.repo_path(&Text::of("/check-runs")),
                body,
            )
            .map(|_reply| ()),
    }
}

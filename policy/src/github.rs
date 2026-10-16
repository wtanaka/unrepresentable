//! A small GitHub REST and GraphQL client over `gh api`. The token comes from
//! the environment of the single step that needs it and is handed to `gh`
//! through its environment, never through argv or disk.

use crate::bytes::Bytes;
use crate::failure::Failure;
use crate::json::{self, Json};
use crate::process::{self, Args, EnvPair, Invocation};
use crate::types::{Answer, Message, Name, Text, Token};

/// The number of items requested per page.
const PAGE_SIZE: usize = 100;
/// The most pages read for one listing.
const MAX_PAGES: usize = 20;
/// The most items a listing may have: twenty pages of a hundred.
const MAX_ITEMS: usize = 2_000;
/// The Accept header for JSON.
const ACCEPT_JSON: &str = "Accept: application/vnd.github+json";
/// The Accept header for raw file contents.
const ACCEPT_RAW: &str = "Accept: application/vnd.github.raw+json";
/// The header that pins the API version.
const API_VERSION: &str = "X-GitHub-Api-Version: 2022-11-28";
/// The most characters of an error body that are kept.
const ERROR_LIMIT: usize = 200;

/// A write method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Method {
    /// The PATCH method.
    Patch,
    /// The POST method.
    Post,
    /// The PUT method.
    Put,
}

impl Method {
    /// The method name.
    pub(crate) fn name(self) -> Text {
        Text::of(match self {
            Self::Post => "POST",
            Self::Patch => "PATCH",
            Self::Put => "PUT",
        })
    }
}

/// Access to one repository's API.
pub(crate) trait Github {
    /// Calls `GET path` and returns the JSON answer.
    fn get(&self, path: &Text) -> Result<Json, Failure>;

    /// The response body as text, asking for the raw media type.
    fn get_text(&self, path: &Text) -> Result<Text, Failure>;

    /// Runs a GraphQL query and returns its `data`.
    fn graphql(&self, query: &Text, variables: &Json) -> Result<Json, Failure>;

    /// Every item of a paged listing, up to twenty pages of a hundred. With a
    /// `key` the listing is that member of each page object. A listing that
    /// is still full after the last page is an error, not a shorter list, and
    /// so is a page that is not an array (or has no such member): a decision
    /// made on part of a listing must not look like one made on all of it.
    fn paged(&self, path: &Text, key: Option<&Name>) -> Result<Vec<Json>, Failure> {
        let separator = if path.as_str().contains('?') {
            '&'
        } else {
            '?'
        };
        let mut out = Vec::new();
        for page in 1..=MAX_PAGES {
            let chunk = self.get(&Text::from_string(format!(
                "{}{separator}per_page={PAGE_SIZE}&page={page}",
                path.as_str()
            )))?;
            let listed = match key {
                Some(member) => chunk.get(member).and_then(Json::as_array),
                None => chunk.as_array(),
            };
            let Some(found) = listed else {
                return Err(Failure::api(&Message::from_string(format!(
                    "{}: page {page} is not a list",
                    route_of(path).as_str()
                ))));
            };
            let count = found.len();
            out.extend(found.iter().cloned());
            if count < PAGE_SIZE {
                return Ok(out);
            }
        }
        Err(Failure::api(&Message::from_string(format!(
            "{}: more than {MAX_ITEMS} items; not reading a part of the list",
            route_of(path).as_str()
        ))))
    }

    /// `owner/name`.
    fn repo(&self) -> Text;

    /// `/repos/<repo><suffix>`.
    fn repo_path(&self, suffix: &Text) -> Text {
        Text::from_string(format!(
            "/repos/{}{}",
            self.repo().as_str(),
            suffix.as_str()
        ))
    }

    /// Sends `method path` with a JSON body and returns the JSON answer.
    fn send(&self, method: Method, path: &Text, body: &Json) -> Result<Json, Failure>;
}

/// The client that shells out to `gh api` with one token.
pub(crate) struct GhClient {
    /// The repository, as `owner/name`.
    repo: Text,
    /// The token to send.
    token: Token,
}

/// One `gh api` call.
struct Call<'call> {
    /// The media type to ask for.
    accept: Media,
    /// The request body, when there is one.
    body: Option<Bytes>,
    /// The path to call.
    path: &'call Text,
    /// The HTTP method.
    verb: Text,
}

/// Which representation of a resource to ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Media {
    /// JSON.
    Json,
    /// Raw file contents.
    Raw,
}

impl Media {
    /// The Accept header for this media type.
    fn header(self) -> Text {
        Text::of(match self {
            Self::Json => ACCEPT_JSON,
            Self::Raw => ACCEPT_RAW,
        })
    }
}

impl GhClient {
    /// Makes one call and returns the response body.
    fn call(&self, request: &Call<'_>) -> Result<Text, Failure> {
        let mut args = Args::of(&["api", "--method", request.verb.as_str()])
            .with(request.path.as_str())
            .with("-H")
            .with(request.accept.header().as_str())
            .with("-H")
            .with(API_VERSION);
        if request.body.is_some() {
            args = args.with("--input").with("-");
        }
        let program = Name::of("gh");
        let mut invocation = Invocation::new(&program, &args)
            .with_env(&EnvPair {
                name: Name::of("GH_TOKEN"),
                value: Text::of(self.token.as_str()),
            })
            .with_env(&EnvPair {
                name: Name::of("GH_PROMPT_DISABLED"),
                value: Text::of("1"),
            })
            .with_env(&EnvPair {
                name: Name::of("GH_NO_UPDATE_NOTIFIER"),
                value: Text::of("1"),
            })
            .without_env(&Name::of("GH_ENTERPRISE_TOKEN"));
        if let Some(body) = &request.body {
            invocation = invocation.feeding(body);
        }
        let captured = process::run(&invocation)?;
        if captured.succeeded() == Answer::Yes {
            return Ok(captured.stdout.lossy_text());
        }
        let route = match request.path.as_str().split_once('?') {
            Some((before, _)) => before,
            None => request.path.as_str(),
        };
        let reason: String = captured
            .stderr
            .lossy_text()
            .as_str()
            .trim()
            .chars()
            .take(ERROR_LIMIT)
            .collect();
        Err(Failure::api(&Message::from_string(format!(
            "{} {route} -> {reason}",
            request.verb.as_str()
        ))))
    }

    /// A client for `repo` that sends `token`.
    pub(crate) fn new(token: &Token, repo: &Text) -> Self {
        Self {
            repo: repo.clone(),
            token: token.clone(),
        }
    }

    /// Parses a response body as JSON.
    fn parse(body: &Text) -> Result<Json, Failure> {
        if body.as_str().trim().is_empty() {
            return Ok(Json::Null);
        }
        json::parse(body).map_err(|message| Failure::api(&message))
    }
}

impl Github for GhClient {
    fn get(&self, path: &Text) -> Result<Json, Failure> {
        Self::parse(&self.call(&Call {
            accept: Media::Json,
            body: None,
            path,
            verb: Text::of("GET"),
        })?)
    }

    fn get_text(&self, path: &Text) -> Result<Text, Failure> {
        self.call(&Call {
            accept: Media::Raw,
            body: None,
            path,
            verb: Text::of("GET"),
        })
    }

    fn graphql(&self, query: &Text, variables: &Json) -> Result<Json, Failure> {
        let body = Json::object_of(vec![
            (Name::of("query"), Json::text_of(query)),
            (Name::of("variables"), variables.clone()),
        ]);
        let reply = Self::parse(&self.call(&Call {
            accept: Media::Json,
            body: Some(Bytes::of_text(&body.compact())),
            path: &Text::of("/graphql"),
            verb: Text::of("POST"),
        })?)?;
        if let Some(errors) = reply.get(&Name::of("errors")) {
            if errors.as_array().is_some_and(|found| !found.is_empty()) {
                return Err(Failure::api(&Message::of("graphql error")));
            }
        }
        match reply.get(&Name::of("data")) {
            Some(data) => Ok(data.clone()),
            None => Err(Failure::api(&Message::of("graphql reply had no data"))),
        }
    }

    fn repo(&self) -> Text {
        self.repo.clone()
    }

    fn send(&self, method: Method, path: &Text, body: &Json) -> Result<Json, Failure> {
        Self::parse(&self.call(&Call {
            accept: Media::Json,
            body: Some(Bytes::of_text(&body.compact())),
            path,
            verb: method.name(),
        })?)
    }
}

/// The path without its query, for a message.
fn route_of(path: &Text) -> Text {
    match path.as_str().split_once('?') {
        Some((before, _)) => Text::of(before),
        None => path.clone(),
    }
}

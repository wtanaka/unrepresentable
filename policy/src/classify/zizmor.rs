//! Mapping `zizmor --format json` output to reasons. Every message in it is
//! PR data.

use crate::failure::Failure;
use crate::json::{self, Json};
use crate::reason::{Code, Reason};
use crate::render::has_unsafe_chars;
use crate::types::{Answer, Count, Message, Name, RelPath, Text};

/// The most characters of a rule identifier that are kept.
const IDENT_LIMIT: Count = Count::new(64);
/// The most characters of a finding description that are kept.
const DESC_LIMIT: Count = Count::new(200);

/// The member's text (or its JSON spelling), cut to `limit` characters.
fn limited(value: Option<&Json>, fallback: &Text, limit: Count) -> Text {
    let full = match value {
        Some(found) => match found.as_text() {
            Some(text) => text.clone(),
            None => found.compact(),
        },
        None => fallback.clone(),
    };
    Text::from_string(full.as_str().chars().take(limit.get()).collect())
}

/// The rule identifier in lower case.
fn lower_ident(ident: &Text) -> Text {
    Text::from_string(
        ident
            .as_str()
            .to_lowercase()
            .chars()
            .map(|found| {
                if found.is_ascii_lowercase() || found.is_ascii_digit() || found == '-' {
                    found
                } else {
                    '?'
                }
            })
            .collect(),
    )
}

/// The workflow path and zero-based row of a finding's first usable location.
#[must_use]
fn first_location(finding: &Json) -> Option<(Text, Count)> {
    let locations = finding.get(&Name::of("locations"))?.as_array()?;
    for location in locations {
        let found = location_of(location);
        if found.is_some() {
            return found;
        }
    }
    None
}

/// The file and line of a finding location.
#[must_use]
fn location_of(location: &Json) -> Option<(Text, Count)> {
    let path = location
        .get(&Name::of("symbolic"))?
        .get(&Name::of("key"))?
        .get(&Name::of("Local"))?
        .text_at(&Name::of("given_path"))?;
    let row = location
        .get(&Name::of("concrete"))?
        .get(&Name::of("location"))?
        .get(&Name::of("start_point"))?
        .get(&Name::of("row"))?;
    let count = match row.as_text() {
        Some(text) => Count::parse(text)?,
        None => row.as_count()?,
    };
    Some((path.clone(), count))
}

/// The reason a zizmor finding stands for.
fn finding_reason(finding: &Json) -> Result<Reason, Failure> {
    let ident = lower_ident(&limited(
        finding.get(&Name::of("ident")),
        &Text::of("unknown"),
        IDENT_LIMIT,
    ));
    let desc = limited(finding.get(&Name::of("desc")), &Text::of(""), DESC_LIMIT);
    let evidence = format!("{}: {}", ident.as_str(), desc.as_str());
    let Some((given, row)) = first_location(finding) else {
        return Ok(Reason::general(
            Code::ZizmorFinding,
            &Text::from_string(evidence),
        ));
    };
    if given.as_str().is_empty() || has_unsafe_chars(&given) == Answer::Yes {
        return Ok(Reason::general(
            Code::ZizmorFinding,
            &Text::from_string(evidence),
        ));
    }
    let line = row.next()?;
    let path = if given.as_str().starts_with(".github/") {
        given.as_str().to_owned()
    } else {
        format!(".github/{}", given.as_str().trim_start_matches('/'))
    };
    Ok(Reason::at(
        Code::ZizmorFinding,
        &Text::from_string(format!("{evidence} ({path}:{})", line.get())),
        &RelPath::of(&path),
        line,
    ))
}

/// Reasons for `zizmor --format json` output. Output that is empty, is not
/// JSON or is not an array is an error, not "no findings": the workflow runs
/// zizmor with `|| true` (it exits non-zero when it has findings), so a crash
/// or a truncated report reaches here as such text. The error fails the
/// classify job, which the policy decision reads as a blocker that no approval
/// can clear.
pub(crate) fn reasons(json_text: &Text) -> Result<Vec<Reason>, Failure> {
    let Ok(parsed) = json::parse(json_text) else {
        return Err(Failure::parse(&Message::of(
            "zizmor output was empty or not valid JSON",
        )));
    };
    let Some(findings) = parsed.as_array() else {
        return Err(Failure::parse(&Message::of(
            "zizmor output was not a list of findings",
        )));
    };
    let mut out = Vec::new();
    for finding in findings {
        if finding.entries().is_none() {
            continue;
        }
        out.push(finding_reason(finding)?);
    }
    Ok(out)
}

//! Strict line format for per-job result artifacts.
//!
//! Artifacts produced by jobs that touch head code are untrusted input. They
//! are parsed with a size limit and a fixed grammar; anything else is a parse
//! failure, which the policy job turns into a blocking (non-approvable)
//! failure.
//!
//! ```text
//! unrepresentable-policy-result v1
//! job <name>
//! head <40 hex>
//! base <40 hex>
//! R <CODE> <b64 evidence> <b64 file or -> <line>
//! N <key> <non-negative int>
//! end
//! ```

#[cfg(test)]
mod tests;

use crate::bytes::{self, Bytes};
use crate::failure::Failure;
use crate::reason::{Code, Commits, Counts, Job, Reason};
use crate::types::{Answer, Count, Glyph, Holds as _, Message, Name, RelPath, Text};

/// The most bytes a result file may hold.
pub(crate) const MAX_BYTES: usize = 0x1_0000;
/// The most lines a result file may hold.
pub(crate) const MAX_LINES: usize = 1000;
/// The first line of every result file.
const MAGIC: &str = "unrepresentable-policy-result v1";
/// The index of the first reason or count record.
const FIRST_RECORD: usize = 4;
/// The fewest lines a complete result file has.
const MIN_LINES: usize = 5;

/// The shape of a base64 field.
const BASE64_FIELD: Shape = Shape {
    alphabet: Alphabet::Base64,
    longest: Count::new(2048),
    shortest: Count::new(0),
};

/// The shape of a line number field.
const LINE_NUMBER: Shape = Shape {
    alphabet: Alphabet::Digits,
    longest: Count::new(7),
    shortest: Count::new(1),
};

/// The shape of a reason code name.
const CODE_NAME: Shape = Shape {
    alphabet: Alphabet::CodeName,
    longest: Count::new(40),
    shortest: Count::new(3),
};

/// The shape of a count key.
const COUNT_KEY: Shape = Shape {
    alphabet: Alphabet::CountKey,
    longest: Count::new(32),
    shortest: Count::new(1),
};

/// The shape of a count value.
const COUNT_VALUE: Shape = Shape {
    alphabet: Alphabet::Digits,
    longest: Count::new(9),
    shortest: Count::new(1),
};

/// What a successfully parsed artifact holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Parsed {
    /// The counts of findings by name.
    pub counts: Counts,
    /// The reasons the result records.
    pub reasons: Vec<Reason>,
}

/// Why an artifact was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    /// A reason code that the job may not report.
    ForeignCode(Code, Job),
    /// The header line is wrong.
    Header,
    /// The recorded line count disagrees with the file.
    LineCount,
    /// The end marker is missing.
    NoEnd,
    /// The file does not end with a newline.
    NoFinalNewline,
    /// The file holds a byte outside ASCII.
    NotAscii,
    /// The file exceeds the size or line limit.
    TooLarge,
    /// A record could not be parsed.
    Unparsable,
}

impl Refusal {
    /// The failure this problem is reported as.
    fn failure(self) -> Failure {
        Failure::parse(&Message::from_string(match self {
            Self::TooLarge => format!("artifact larger than {MAX_BYTES} bytes"),
            Self::NotAscii => "non-ASCII artifact".to_owned(),
            Self::NoFinalNewline => "missing final newline".to_owned(),
            Self::LineCount => "line count out of range".to_owned(),
            Self::Header => "header does not match the evaluated run".to_owned(),
            Self::NoEnd => "missing end line".to_owned(),
            Self::Unparsable => "unparsable line".to_owned(),
            Self::ForeignCode(code, job) => format!(
                "job {} may not emit {}",
                job.name().as_str(),
                code.name().as_str()
            ),
        }))
    }
}

/// The characters a token may contain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Alphabet {
    /// Base64 digits.
    Base64,
    /// Characters of a reason code name.
    CodeName,
    /// Characters of a count key.
    CountKey,
    /// Decimal digits.
    Digits,
}

impl Alphabet {
    /// Whether `found` belongs to the alphabet.
    fn admits(self, found: Glyph) -> Answer {
        let ch = found.get();
        match self {
            Self::Base64 => ch.is_ascii_alphanumeric() || matches!(ch, '+' | '/' | '='),
            Self::Digits => ch.is_ascii_digit(),
            Self::CountKey => ch.is_ascii_lowercase() || ch == '_',
            Self::CodeName => ch.is_ascii_uppercase() || ch == '_',
        }
        .answer()
    }
}

/// An alphabet and an inclusive length range.
struct Shape {
    /// The characters the field may use.
    alphabet: Alphabet,
    /// The most characters the field may have.
    longest: Count,
    /// The fewest characters the field may have.
    shortest: Count,
}

impl Shape {
    /// Whether `token` has an admitted length and alphabet.
    fn fits(&self, token: &Text) -> Answer {
        let length = token.as_str().chars().count();
        (length >= self.shortest.get()
            && length <= self.longest.get()
            && token
                .as_str()
                .chars()
                .all(|found| self.alphabet.admits(Glyph::new(found)) == Answer::Yes))
        .answer()
    }
}

/// The four header lines for `job` and `commits`.
fn header(job: Job, commits: &Commits) -> [Text; FIRST_RECORD] {
    [
        Text::of(MAGIC),
        Text::from_string(format!("job {}", job.name().as_str())),
        Text::from_string(format!("head {}", commits.head.as_str())),
        Text::from_string(format!("base {}", commits.base.as_str())),
    ]
}

/// Renders the artifact text for `job`.
pub(crate) fn emit(job: Job, commits: &Commits, reasons: &[Reason], counts: &Counts) -> Text {
    let mut lines: Vec<String> = header(job, commits)
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect();
    for reason in reasons {
        let evidence = bytes::encode(&Bytes::of_text(&reason.evidence));
        let path = match &reason.path {
            Some(found) => bytes::encode(&Bytes::of_text(&Text::of(found.as_str()))),
            None => Text::of("-"),
        };
        lines.push(format!(
            "R {} {} {} {}",
            reason.code.name().as_str(),
            evidence.as_str(),
            path.as_str(),
            reason.line.get()
        ));
    }
    for (key, value) in counts.entries() {
        lines.push(format!("N {} {}", key.as_str(), value.get()));
    }
    lines.push("end".to_owned());
    Text::from_string(format!("{}\n", lines.join("\n")))
}

/// Decodes a base64 field into text.
fn decode_field(token: &Text) -> Result<Text, Failure> {
    if BASE64_FIELD.fits(token) == Answer::No {
        return Err(Refusal::Unparsable.failure());
    }
    Ok(bytes::decode(token)?.lossy_text())
}

/// Splits a record line into space-separated tokens.
fn tokens(line: &Text) -> Vec<Text> {
    line.as_str().split(' ').map(Text::of).collect()
}

/// Parses one reason record, or `None` for a count record.
fn parse_reason(line: &Text, job: Job) -> Result<Option<Reason>, Failure> {
    let parts = tokens(line);
    let [marker, code, evidence, path, number] = parts.as_slice() else {
        return Ok(None);
    };
    let starts_upper = code
        .as_str()
        .chars()
        .next()
        .is_some_and(|found| found.is_ascii_uppercase());
    if marker.as_str() != "R"
        || !starts_upper
        || CODE_NAME.fits(code) == Answer::No
        || LINE_NUMBER.fits(number) == Answer::No
    {
        return Ok(None);
    }
    let allowed = match Code::parse(code) {
        Some(found) if found.emitter() == job => found,
        Some(other) => return Err(Refusal::ForeignCode(other, job).failure()),
        None => return Err(Refusal::Unparsable.failure()),
    };
    let decoded = decode_field(evidence)?;
    let located = if path.as_str() == "-" {
        None
    } else {
        Some(RelPath::of(decode_field(path)?.as_str()))
    };
    Ok(Count::parse(number).map(|line_number| Reason {
        code: allowed,
        evidence: decoded,
        line: line_number,
        path: located,
    }))
}

/// Parses one count record into its key and value.
#[must_use]
fn parse_count(line: &Text) -> Option<(Name, Count)> {
    let parts = tokens(line);
    let [marker, key, number] = parts.as_slice() else {
        return None;
    };
    if marker.as_str() != "N"
        || COUNT_KEY.fits(key) == Answer::No
        || COUNT_VALUE.fits(number) == Answer::No
    {
        return None;
    }
    Count::parse(number).map(|value| (Name::of(key.as_str()), value))
}

/// Parses artifact `data` produced by `job` for `commits`.
pub(crate) fn parse(data: &Bytes, job: Job, commits: &Commits) -> Result<Parsed, Failure> {
    if data.len().get() > MAX_BYTES {
        return Err(Refusal::TooLarge.failure());
    }
    if !data.as_slice().is_ascii() {
        return Err(Refusal::NotAscii.failure());
    }
    let Some(whole) = data.strict_text() else {
        return Err(Refusal::NotAscii.failure());
    };
    let Some(body) = whole.as_str().strip_suffix('\n') else {
        return Err(Refusal::NoFinalNewline.failure());
    };
    let lines: Vec<&str> = body.split('\n').collect();
    if lines.len() > MAX_LINES || lines.len() < MIN_LINES {
        return Err(Refusal::LineCount.failure());
    }
    let expected = header(job, commits);
    let matches_header = lines
        .iter()
        .zip(expected.iter())
        .all(|(got, want)| *got == want.as_str());
    if !matches_header {
        return Err(Refusal::Header.failure());
    }
    if lines.last() != Some(&"end") {
        return Err(Refusal::NoEnd.failure());
    }
    let last = Count::new(lines.len()).minus(Count::new(1))?;
    let Some(records) = lines.get(FIRST_RECORD..last.get()) else {
        return Err(Refusal::LineCount.failure());
    };
    let mut reasons = Vec::new();
    let mut counts = Counts::empty();
    for record in records {
        let line = Text::of(record);
        if let Some(reason) = parse_reason(&line, job)? {
            reasons.push(reason);
        } else if let Some((key, value)) = parse_count(&line) {
            if counts.contains(&key) == Answer::Yes {
                return Err(Refusal::Unparsable.failure());
            }
            counts = counts.with(&key, value);
        } else {
            return Err(Refusal::Unparsable.failure());
        }
    }
    Ok(Parsed { counts, reasons })
}

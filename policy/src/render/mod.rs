//! The one rendering function for every string that came from a pull request.
//!
//! Everything a PR can influence (paths, identifiers, corpus item names,
//! zizmor messages) is rendered through `render`. Nothing else may put such a
//! string into a comment, check title, summary or annotation.

#[cfg(test)]
mod tests;

use crate::types::{Answer, Count, Glyph, Holds as _, Text};

/// The most characters a rendered text shows.
pub(crate) const MAX_CHARS: usize = 200;

/// Inclusive ranges of characters that must never reach a reader or an agent
/// unescaped. U+FFFD stands for bytes that were not UTF-8 (a lone surrogate
/// after `surrogateescape` in other languages). Rust strings cannot hold lone
/// surrogates, so U+FFFD, which is what non-UTF-8 path bytes decode to, is
/// treated as unsafe. The invisible fillers (U+034F, U+115F, U+1160, U+3164,
/// U+FFA0), the Mongolian and emoji variation selectors (U+180B to U+180E,
/// U+FE00 to U+FE0F, U+E0100 to U+E01EF) and the invisible format controls
/// (U+2065, U+206A to U+206F) render as nothing, so they could hide text.
const UNSAFE_RANGES: [(u32, u32); 23] = [
    (0x0000, 0x001F),
    (0x007F, 0x009F),
    (0x00AD, 0x00AD),
    (0x034F, 0x034F),
    (0x061C, 0x061C),
    (0x115F, 0x1160),
    (0x180B, 0x180E),
    (0x200B, 0x200F),
    (0x2028, 0x2029),
    (0x202A, 0x202E),
    (0x2060, 0x2064),
    (0x2065, 0x2065),
    (0x2066, 0x2069),
    (0x206A, 0x206F),
    (0x3164, 0x3164),
    (0xFE00, 0xFE0F),
    (0xFEFF, 0xFEFF),
    (0xFFA0, 0xFFA0),
    (0xFFF9, 0xFFFB),
    (0xFFFD, 0xFFFD),
    (0xE0000, 0xE007F),
    (0xE0100, 0xE01EF),
    (0xD800, 0xDFFF),
];

/// A vertical bar breaks a Markdown table cell even inside a code span.
const TABLE_BAR: char = '|';

/// Whether `found` falls in a range that is unsafe to print.
fn in_unsafe_ranges(found: Glyph) -> Answer {
    let code = u32::from(found.get());
    UNSAFE_RANGES
        .iter()
        .any(|(low, high)| *low <= code && code <= *high)
        .answer()
}

/// Whether `found` must be written as an escape.
fn needs_escape(found: Glyph) -> Answer {
    (found.get() == TABLE_BAR || in_unsafe_ranges(found) == Answer::Yes).answer()
}

/// True when a path must be classified `UNSAFE_PATH_CHARS` (excludes `|`).
pub(crate) fn has_unsafe_chars(text: &Text) -> Answer {
    text.as_str()
        .chars()
        .any(|found| in_unsafe_ranges(Glyph::new(found)) == Answer::Yes)
        .answer()
}

/// Escapes the characters of `text` that are not safe to print.
fn escape(text: &Text) -> Text {
    Text::from_string(
        text.as_str()
            .chars()
            .map(|found| {
                if needs_escape(Glyph::new(found)) == Answer::Yes {
                    format!("\\u{{{:04X}}}", u32::from(found))
                } else {
                    found.to_string()
                }
            })
            .collect(),
    )
}

/// Inline code span whose fence is longer than any backtick run inside.
fn code_span(text: &Text) -> Text {
    let body = text.as_str();
    let longest = body
        .split(|found| found != '`')
        .map(str::len)
        .fold(0, usize::max);
    let fence = format!("`{}", "`".repeat(longest));
    let pad = if body.starts_with('`') || body.ends_with('`') || body.is_empty() {
        " "
    } else {
        ""
    };
    Text::from_string(format!("{fence}{pad}{body}{pad}{fence}"))
}

/// `text` cut to `limit` characters, with a note when it was cut.
fn shown(text: &Text, limit: Count) -> Text {
    Text::from_string(text.as_str().chars().take(limit.get()).collect())
}

/// The note that says how much of `text` was cut.
fn truncation_note(text: &Text, limit: Count) -> Text {
    let original = text.as_str().chars().count();
    if original > limit.get() {
        Text::from_string(format!(" (truncated, original length {original})"))
    } else {
        Text::of("")
    }
}

/// Escape, truncate to `limit` characters of the original, code-span.
pub(crate) fn render_limited(value: &Text, limit: Count) -> Text {
    let span = code_span(&escape(&shown(value, limit)));
    Text::from_string(format!(
        "{}{}",
        span.as_str(),
        truncation_note(value, limit).as_str()
    ))
}

/// `text` made safe to print and cut to the limit.
pub(crate) fn render(value: &Text) -> Text {
    render_limited(value, Count::new(MAX_CHARS))
}

/// Like `render` but without the Markdown fence: for plain-text fields such as
/// annotation messages, where the surrounding text is ours.
pub(crate) fn render_text(value: &Text) -> Text {
    let limit = Count::new(MAX_CHARS);
    Text::from_string(format!(
        "{}{}",
        escape(&shown(value, limit)).as_str(),
        truncation_note(value, limit).as_str()
    ))
}

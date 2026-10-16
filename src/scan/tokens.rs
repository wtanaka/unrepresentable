//! Token-level scan for macro bodies that do not parse. Comments are gone
//! and string literals are single tokens, so this is not text matching.
use proc_macro2::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};

use syn::punctuated::Punctuated;

use super::exprs;

use super::items;

use super::paths;

use super::paths::PathRole;

use super::types;

use super::{AssertionSeen, Body, Scanner};

use crate::attrs;

use crate::index::Kind;

use crate::loader::Scope;

use crate::rules;

use crate::types::{Answer, Count, Glyph, Holds as _, Name};

use crate::types::{Message, Text};

use crate::util::skip_items;

/// What the previous token was, used to tell statement starts from member accesses.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Prev {
    /// A `$`, so the next identifier is a macro metavariable.
    Dollar,
    /// A `.`, so the next identifier is a member or method name.
    Dot,
    /// A `let` keyword.
    Let,
    /// Any other token.
    Other,
    /// A `;`, so the next token starts a statement.
    Semi,
    /// Nothing yet: the start of the token stream.
    Start,
}

/// The token being scanned and everything after it.
struct Cursor<'tok> {
    /// The tokens starting at the current token.
    rest: &'tok [TokenTree],
    /// The tokens after the current token.
    tail: &'tok [TokenTree],
}

/// Whether the next fn follows a test attribute.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pending {
    /// No pending test attribute.
    Plain,
    /// A test attribute was just seen, so the next fn is a test.
    TestAttr,
}

/// The scan resumes here: the tokens still to read and what the last one was.
type Resume<'tok> = (&'tok [TokenTree], Prev);

/// Tracks a closure or async block in a token slice. The token scan cannot tell
/// where a closure body ends, so it takes the rest of the statement or list item
/// (up to the next `;` or `,` at the same level) as the body: a `return` there is
/// not reported.
struct NestedBody {
    /// Whether a `Body::Other` entry was pushed for a closure or async block.
    open: Answer,
    /// Whether the previous token ended a value, so a `|` after it is an operator.
    value_ended: Answer,
}

impl NestedBody {
    /// Leaves the closure body, if one is open, at the end of the slice.
    fn finish(&self, scanner: &mut Scanner<'_>) {
        if self.open == Answer::Yes {
            scanner.leave_body();
        }
    }

    /// A tracker at the start of a token slice.
    const fn new() -> Self {
        Self {
            open: Answer::No,
            value_ended: Answer::No,
        }
    }

    /// Updates the tracker for the next token of the slice.
    fn on_token(&mut self, scanner: &mut Scanner<'_>, token: &TokenTree) {
        let starts = match token {
            TokenTree::Punct(punct) => punct.as_char() == '|' && self.value_ended == Answer::No,
            TokenTree::Ident(ident) => ident == "async",
            TokenTree::Group(_) | TokenTree::Literal(_) => false,
        };
        let separator = match token {
            TokenTree::Punct(punct) => matches!(punct.as_char(), ';' | ','),
            TokenTree::Ident(_) | TokenTree::Group(_) | TokenTree::Literal(_) => false,
        };
        if starts && self.open == Answer::No {
            scanner.bodies.push(Body::Other);
            self.open = Answer::Yes;
        }
        if separator && self.open == Answer::Yes {
            scanner.leave_body();
            self.open = Answer::No;
        }
        self.value_ended = match token {
            TokenTree::Ident(ident) => (ident != "move" && ident != "return").answer(),
            TokenTree::Literal(_) | TokenTree::Group(_) => Answer::Yes,
            TokenTree::Punct(_) => Answer::No,
        };
    }
}

/// Does a token after `prev` start a statement?
const fn at_statement_start(prev: Prev) -> Answer {
    match prev {
        Prev::Start | Prev::Semi => Answer::Yes,
        Prev::Dot | Prev::Dollar | Prev::Let | Prev::Other => Answer::No,
    }
}

/// Is the next identifier a member name or macro metavariable, not a path?
const fn after_member_or_meta(prev: Prev) -> Answer {
    match prev {
        Prev::Dot | Prev::Dollar => Answer::Yes,
        Prev::Start | Prev::Semi | Prev::Let | Prev::Other => Answer::No,
    }
}

/// Do the tokens after `return` start with a path ending in `Err` and its
/// parenthesised arguments (`Err(e)`, `Result::Err(e)`, `core::result::Result::Err(e)`)?
#[must_use]
fn returns_err_call(tail: &[TokenTree]) -> Answer {
    let mut last: Option<&Ident> = None;
    for tree in tail {
        match tree {
            TokenTree::Ident(ident) => last = Some(ident),
            TokenTree::Punct(punct) if punct.as_char() == ':' => {}
            TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => {
                return last.is_some_and(|name| name == "Err").answer();
            }
            TokenTree::Punct(_) | TokenTree::Group(_) | TokenTree::Literal(_) => {
                return Answer::No;
            }
        }
    }
    Answer::No
}

/// Returns the head token as a `Punct` if it is the punctuation character `wanted`.
#[must_use]
fn is_punct(tree: Option<&TokenTree>, wanted: Glyph) -> Option<&Punct> {
    match tree {
        Some(TokenTree::Punct(punct)) if punct.as_char() == wanted.get() => Some(punct),
        _ => None,
    }
}

/// Returns the head token as an `Ident`, if it is one.
#[must_use]
const fn as_ident(tree: Option<&TokenTree>) -> Option<&Ident> {
    match tree {
        Some(TokenTree::Ident(ident)) => Some(ident),
        _ => None,
    }
}

/// Returns the head token as a `Group` if it uses the given delimiter.
#[must_use]
fn as_group(tree: Option<&TokenTree>, delimiter: Delimiter) -> Option<&Group> {
    match tree {
        Some(TokenTree::Group(group)) if group.delimiter() == delimiter => Some(group),
        _ => None,
    }
}

/// Splits off `ident :: ident :: ...` starting at `rest`'s head.
fn take_path(rest: &[TokenTree]) -> (Vec<Ident>, &[TokenTree]) {
    let mut idents = Vec::new();
    let mut cursor = rest;
    while let Some((head, tail)) = cursor.split_first() {
        let TokenTree::Ident(ident) = head else {
            break;
        };
        idents.push(ident.clone());
        cursor = tail;
        let colons = is_punct(cursor.first(), Glyph::new(':'))
            .is_some_and(|punct| punct.spacing() == Spacing::Joint)
            && is_punct(cursor.get(1), Glyph::new(':')).is_some()
            && as_ident(cursor.get(2)).is_some();
        if colons {
            cursor = skip_items(cursor, Count::new(2));
        } else {
            break;
        }
    }
    (idents, cursor)
}

/// Builds a `syn::Path` from identifiers, with no leading colon or generics.
fn syn_path(idents: &[Ident]) -> syn::Path {
    let mut segments = Punctuated::new();
    for ident in idents {
        segments.push(syn::PathSegment::from(ident.clone()));
    }
    syn::Path {
        leading_colon: None,
        segments,
    }
}

/// Token-level scan of a body that parses as no items, statements or expressions.
/// It always reports `unparsed-macro`, so the gap (most rules did not run) is visible.
///
/// A body with no group, no literal and at most one identifier (`Token![,]`,
/// `Token![if]`) holds nothing any rule could see, so it is not reported.
pub(crate) fn scan_unparsed(scanner: &mut Scanner<'_>, tokens: TokenStream, span: Span) {
    if is_inert(&tokens) == Answer::Yes {
        return;
    }
    scan_tokens(scanner, tokens);
    scanner.report(
        span,
        rules::UNPARSED_MACRO,
        &Message::of(
            "macro body did not parse as items, statements or expressions; only the token-level scan ran",
        ),
    );
}

/// Is `tokens` only punctuation and at most one identifier?
fn is_inert(tokens: &TokenStream) -> Answer {
    let mut idents = Count::new(0);
    for tree in tokens.clone() {
        match tree {
            TokenTree::Punct(_) => {}
            TokenTree::Ident(_) => match idents.next() {
                Ok(next) => idents = next,
                Err(_) => return Answer::No,
            },
            TokenTree::Group(_) | TokenTree::Literal(_) => return Answer::No,
        }
    }
    (idents.get() <= 1).answer()
}

/// Scans a token stream for rule violations, recursing into groups.
pub(crate) fn scan_tokens(scanner: &mut Scanner<'_>, tokens: TokenStream) {
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    scan_slice(scanner, &trees);
}

/// Scans a token slice one token at a time, tracking the previous token and any test attribute.
fn scan_slice(scanner: &mut Scanner<'_>, trees: &[TokenTree]) {
    let mut rest = trees;
    let mut prev = Prev::Start;
    let mut pending = Pending::Plain;
    let mut nested = NestedBody::new();
    while let Some((head, tail)) = rest.split_first() {
        nested.on_token(scanner, head);
        match head {
            TokenTree::Punct(punct) => {
                prev = on_punct(scanner, punct, tail, prev, &mut pending);
                if punct.as_char() == '#' {
                    if let Some(after) =
                        on_attribute_tokens(scanner, tail, punct.span(), &mut pending)
                    {
                        rest = after;
                        prev = Prev::Other;
                        continue;
                    }
                }
                rest = tail;
            }
            TokenTree::Group(group) => {
                scan_tokens(scanner, group.stream());
                prev = Prev::Other;
                rest = tail;
            }
            TokenTree::Literal(literal) => {
                let text = literal.to_string();
                let float = match syn::parse_str::<syn::Lit>(&text) {
                    Ok(syn::Lit::Float(_)) => true,
                    Ok(syn::Lit::Int(int)) => matches!(int.suffix(), "f32" | "f64"),
                    _ => false,
                };
                if float {
                    scanner.report(
                        literal.span(),
                        rules::FLOAT_TYPE,
                        &Message::of("float literal"),
                    );
                }
                prev = Prev::Other;
                rest = tail;
            }
            TokenTree::Ident(ident) => {
                let (next_rest, next_prev) =
                    on_ident(scanner, ident, &Cursor { rest, tail }, (prev, &mut pending));
                rest = next_rest;
                prev = next_prev;
            }
        }
    }
    nested.finish(scanner);
}

/// Handles a punctuation token: method calls after `.`, and updates the previous-token state.
fn on_punct(
    scanner: &mut Scanner<'_>,
    punct: &Punct,
    tail: &[TokenTree],
    prev: Prev,
    _pending: &mut Pending,
) -> Prev {
    match punct.as_char() {
        '.' if punct.spacing() == Spacing::Alone => {
            if let Some(name) = as_ident(tail.first()) {
                let called = as_group(tail.get(1), Delimiter::Parenthesis).is_some()
                    || is_punct(tail.get(1), Glyph::new(':')).is_some();
                if called {
                    paths::check_method_name(
                        scanner,
                        name.span(),
                        &Name::from_string(name.to_string()),
                    );
                }
            }
            Prev::Dot
        }
        '$' => Prev::Dollar,
        ';' => Prev::Semi,
        '=' if punct.spacing() == Spacing::Alone && prev == Prev::Let => Prev::Other,
        _ => Prev::Other,
    }
}

/// Handles `#[..]` and `#![..]`; returns the slice after the attribute.
#[must_use]
fn on_attribute_tokens<'tok>(
    scanner: &mut Scanner<'_>,
    tail: &'tok [TokenTree],
    pound: Span,
    pending: &mut Pending,
) -> Option<&'tok [TokenTree]> {
    let (after_bang, group_tree) = if is_punct(tail.first(), Glyph::new('!')).is_some() {
        (tail.get(1..)?, tail.get(1))
    } else {
        (tail, tail.first())
    };
    let group = as_group(group_tree, Delimiter::Bracket)?;
    let after = after_bang.get(1..)?;
    match syn::parse2::<syn::Meta>(group.stream()) {
        Ok(meta) => {
            let mut test = false;
            attrs::expand_meta(&meta, &[], &mut |expanded, preds| {
                if items::is_test_path(scanner, expanded.path()) == Answer::Yes {
                    test = true;
                }
                items::on_expanded_meta(scanner, expanded, preds, pound);
            });
            if test {
                *pending = Pending::TestAttr;
            }
        }
        Err(_) => scan_attribute_fallback(scanner, group.stream(), pound),
    }
    Some(after)
}

/// Reports `allow`, `warn` and `expect` attributes in an attribute body that did not parse.
fn scan_attribute_fallback(scanner: &mut Scanner<'_>, stream: TokenStream, pound: Span) {
    let trees: Vec<TokenTree> = stream.into_iter().collect();
    let mut rest = trees.as_slice();
    while let Some((head, tail)) = rest.split_first() {
        if let TokenTree::Ident(ident) = head {
            let name = ident.to_string();
            if matches!(name.as_str(), "allow" | "warn" | "expect")
                && as_group(tail.first(), Delimiter::Parenthesis).is_some()
            {
                scanner.report(
                    pound,
                    rules::SUPPRESSION,
                    &Message::from_string(format!("`#[{name}(..)]` inside an unparsed macro body")),
                );
            }
        }
        if let TokenTree::Group(group) = head {
            scan_attribute_fallback(scanner, group.stream(), pound);
        }
        rest = tail;
    }
}

/// Keywords and operators that matter by themselves: `fn`, `let`, `_ =`, `as`, floats.
/// `Some` when the identifier has been fully handled.
#[must_use]
fn on_keyword<'tok>(
    scanner: &mut Scanner<'_>,
    ident: &Ident,
    cursor: &Cursor<'tok>,
    state: (Prev, &mut Pending),
) -> Option<Resume<'tok>> {
    let (prev, pending) = state;
    let tail = cursor.tail;
    let name = ident.to_string();
    match name.as_str() {
        "return" => exprs::on_return(scanner, ident.span(), returns_err_call(tail)),
        "fn" if as_ident(tail.first()).is_some() => {
            if let Some(after) = on_fn_group(scanner, tail, *pending) {
                *pending = Pending::Plain;
                return Some((after, Prev::Other));
            }
        }
        "let" => {
            if as_ident(tail.first()).is_some_and(|next| next == "_") {
                scanner.report(
                    ident.span(),
                    rules::DISCARDED_VALUE,
                    &Message::of("`let _ = ..` discards a value; handle it"),
                );
            }
            return Some((tail, Prev::Let));
        }
        "_" if at_statement_start(prev) == Answer::Yes => {
            if is_punct(tail.first(), Glyph::new('='))
                .is_some_and(|punct| punct.spacing() == Spacing::Alone)
            {
                scanner.report(
                    ident.span(),
                    rules::DISCARDED_VALUE,
                    &Message::of("`_ = ..` discards a value; handle it"),
                );
            }
        }
        "as" => {
            if let Some(target) = as_ident(tail.first()) {
                let target_text = target.to_string();
                if target_text == "_" || rules::NUMERIC_PRIMITIVES.contains(&target_text.as_str()) {
                    scanner.report(
                        ident.span(),
                        rules::NUMERIC_CAST,
                        &Message::of(
                            "`as` cast to a numeric type; use TryFrom or a checked conversion",
                        ),
                    );
                }
            }
        }
        "f32" | "f64" => {
            scanner.report(ident.span(), rules::FLOAT_TYPE, &Message::of("float type"));
        }
        _ => {}
    }
    None
}

/// A path ending in `!` and a group: records an assertion macro and checks its name.
fn on_macro_call<'tok>(
    scanner: &mut Scanner<'_>,
    idents: &[Ident],
    after: &'tok [TokenTree],
) -> Resume<'tok> {
    let path = syn_path(idents);
    paths::check_macro_name(scanner, &path);
    if let Some(leaf) = idents.last() {
        let leaf_text = leaf.to_string();
        if rules::TEST_ASSERTION_MACROS.contains(&leaf_text.as_str())
            || scanner
                .env
                .config
                .assertion_macros
                .iter()
                .any(|extra| extra.as_str() == leaf_text)
        {
            if let Some(slot) = scanner.assertions.last_mut() {
                *slot = AssertionSeen::Yes;
            }
        }
    }
    (skip_items(after, Count::new(1)), Prev::Other)
}

/// A plain path expression: resolved, then checked as a path and a method name.
fn on_path(scanner: &mut Scanner<'_>, idents: &[Ident]) {
    let segments: Vec<Name> = idents
        .iter()
        .map(|segment| Name::from_string(segment.to_string()))
        .collect();
    let resolved = types::resolve_segments(scanner, &segments);
    let interesting = segments.len() > 1 || resolved.kind != Kind::Unresolved;
    if !interesting {
        return;
    }
    if let Some(first) = idents.first() {
        paths::check_resolved(scanner, &resolved, first.span(), PathRole::Expr);
    }
    if segments.len() > 1 {
        if let Some(last) = idents.last() {
            paths::check_method_name(scanner, last.span(), &Name::from_string(last.to_string()));
        }
    }
}

/// Handles an identifier token: keywords, macro calls, then plain paths.
fn on_ident<'tok>(
    scanner: &mut Scanner<'_>,
    ident: &Ident,
    cursor: &Cursor<'tok>,
    state: (Prev, &mut Pending),
) -> Resume<'tok> {
    let prev = state.0;
    if let Some(done) = on_keyword(scanner, ident, cursor, state) {
        return done;
    }
    if after_member_or_meta(prev) == Answer::Yes {
        return (cursor.tail, Prev::Other);
    }
    let (idents, after) = take_path(cursor.rest);
    let macro_call = is_punct(after.first(), Glyph::new('!'))
        .is_some_and(|punct| punct.spacing() == Spacing::Alone)
        && matches!(after.get(1), Some(TokenTree::Group(_)));
    if macro_call {
        return on_macro_call(scanner, &idents, after);
    }
    on_path(scanner, &idents);
    (after, Prev::Other)
}

/// `fn name (..) {body}`: scanned as a fn. Returns the slice after it.
#[must_use]
fn on_fn_group<'tok>(
    scanner: &mut Scanner<'_>,
    tail: &'tok [TokenTree],
    pending: Pending,
) -> Option<&'tok [TokenTree]> {
    let name = as_ident(tail.first())?;
    let mut cursor = tail.get(1..)?;
    let mut head_tokens: Vec<TokenTree> = Vec::new();
    let mut found_body: Option<&Group> = None;
    while let Some((tree, next)) = cursor.split_first() {
        cursor = next;
        if let TokenTree::Group(group) = tree {
            if group.delimiter() == Delimiter::Brace {
                found_body = Some(group);
                break;
            }
        }
        if is_punct(Some(tree), Glyph::new(';')).is_some() {
            break;
        }
        head_tokens.push(tree.clone());
    }
    let body = found_body?;
    let stream: TokenStream = head_tokens.into_iter().collect();
    scan_tokens(scanner, stream);
    let saved = scanner.scope;
    let is_test = pending == Pending::TestAttr || scanner.scope == Scope::Test;
    if is_test {
        scanner.scope = Scope::Test;
        scanner.assertions.push(AssertionSeen::No);
    }
    scanner.bodies.push(if pending == Pending::TestAttr {
        Body::Test
    } else {
        Body::Other
    });
    scanner.push_item(Text::from_string(format!("fn {name}")));
    scan_tokens(scanner, body.stream());
    scanner.leave_body();
    if is_test && scanner.assertions.pop() == Some(AssertionSeen::No) {
        scanner.report(
            name.span(),
            rules::TEST_ASSERTION,
            &Message::from_string(format!("test fn `{name}` contains no assertion macro")),
        );
    }
    scanner.pop_item();
    scanner.scope = saved;
    Some(cursor)
}

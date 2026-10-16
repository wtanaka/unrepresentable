//! Turns the transcribers of a `macro_rules!` definition into token streams
//! that `syn` can parse: metavariables become placeholders and repetitions
//! become one copy of their body.
use alloc::collections::BTreeMap;

use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream, TokenTree};

use crate::types::{Answer, Holds as _, Name, Text};

/// The fragment kinds of the metavariables of one rule, by metavariable name.
type Kinds = BTreeMap<Name, Name>;

/// The transcriber of every rule of a `macro_rules!` body, metavariables replaced.
/// `None` when the body is not a list of `(matcher) => { transcriber }` rules.
#[must_use]
pub(super) fn transcribers(tokens: TokenStream) -> Option<Vec<TokenStream>> {
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    let mut found = Vec::new();
    let mut rest = trees.as_slice();
    while !rest.is_empty() {
        let (matcher, after_matcher) = take_group(rest)?;
        let after_arrow = take_arrow(after_matcher)?;
        let (transcriber, after_rule) = take_group(after_arrow)?;
        let mut kinds = Kinds::new();
        collect_kinds(matcher.stream(), &mut kinds);
        found.push(replace(transcriber.stream(), &kinds));
        rest = match after_rule.split_first() {
            Some((TokenTree::Punct(semicolon), after)) if semicolon.as_char() == ';' => after,
            Some(_) => return None,
            None => after_rule,
        };
    }
    Some(found)
}

/// Splits a leading group off `trees`.
#[must_use]
const fn take_group(trees: &[TokenTree]) -> Option<(&Group, &[TokenTree])> {
    match trees.split_first() {
        Some((TokenTree::Group(group), rest)) => Some((group, rest)),
        _ => None,
    }
}

/// Skips a leading `=>`.
#[must_use]
fn take_arrow(trees: &[TokenTree]) -> Option<&[TokenTree]> {
    match trees {
        [TokenTree::Punct(equals), TokenTree::Punct(greater), rest @ ..]
            if equals.as_char() == '=' && greater.as_char() == '>' =>
        {
            Some(rest)
        }
        _ => None,
    }
}

/// Records every `$name:kind` of a matcher, inside repetitions too.
fn collect_kinds(stream: TokenStream, kinds: &mut Kinds) {
    let trees: Vec<TokenTree> = stream.into_iter().collect();
    let mut rest = trees.as_slice();
    while let Some((head, tail)) = rest.split_first() {
        match (head, tail) {
            (
                TokenTree::Punct(dollar),
                [TokenTree::Ident(name), TokenTree::Punct(colon), TokenTree::Ident(kind), ..],
            ) if dollar.as_char() == '$' && colon.as_char() == ':' => {
                kinds.extend([(
                    Name::from_string(name.to_string()),
                    Name::from_string(kind.to_string()),
                )]);
            }
            (TokenTree::Group(group), _) => collect_kinds(group.stream(), kinds),
            _ => {}
        }
        rest = tail;
    }
}

/// The text a metavariable of fragment kind `kind` stands for.
fn placeholder_text(kind: Option<&Name>) -> Text {
    Text::of(match kind.map(Name::as_str) {
        Some("ty" | "path" | "pat" | "pat_param") => "Placeholder",
        Some("item") => "struct Placeholder;",
        Some("block") => "{}",
        Some("literal") => "0",
        Some("lifetime") => "'placeholder",
        Some("vis") => "",
        Some("stmt") => "placeholder()",
        Some(_) | None => "placeholder",
    })
}

/// `text` as tokens that all carry `span`, so findings point at the template.
fn tokens_at(text: &Text, span: Span) -> Vec<TokenTree> {
    match text.as_str().parse::<TokenStream>() {
        Ok(stream) => stream.into_iter().map(|tree| respan(tree, span)).collect(),
        Err(_) => Vec::new(),
    }
}

/// `tree` with `span` on it and on everything inside it.
fn respan(tree: TokenTree, span: Span) -> TokenTree {
    match tree {
        TokenTree::Group(group) => {
            let inner: TokenStream = group
                .stream()
                .into_iter()
                .map(|nested| respan(nested, span))
                .collect();
            let mut renewed = Group::new(group.delimiter(), inner);
            renewed.set_span(span);
            TokenTree::Group(renewed)
        }
        TokenTree::Ident(mut ident) => {
            ident.set_span(span);
            TokenTree::Ident(ident)
        }
        TokenTree::Punct(mut punct) => {
            punct.set_span(span);
            TokenTree::Punct(punct)
        }
        TokenTree::Literal(mut literal) => {
            literal.set_span(span);
            TokenTree::Literal(literal)
        }
    }
}

/// Is `tree` a repetition operator (`*`, `+` or `?`)?
fn is_repetition_operator(tree: Option<&TokenTree>) -> Answer {
    matches!(tree, Some(TokenTree::Punct(punct)) if matches!(punct.as_char(), '*' | '+' | '?'))
        .answer()
}

/// What follows a `$( .. )` group: its separator and operator, skipped.
#[must_use]
fn after_repetition(rest: &[TokenTree]) -> &[TokenTree] {
    match rest.split_first() {
        Some((head, tail)) if is_repetition_operator(Some(head)) == Answer::Yes => tail,
        Some((_, tail)) if is_repetition_operator(tail.first()) == Answer::Yes => {
            match tail.get(1..) {
                Some(after) => after,
                None => tail,
            }
        }
        Some(_) | None => rest,
    }
}

/// Replaces the metavariables and repetitions of a transcriber.
fn replace(stream: TokenStream, kinds: &Kinds) -> TokenStream {
    let trees: Vec<TokenTree> = stream.into_iter().collect();
    let mut out: Vec<TokenTree> = Vec::new();
    let mut rest = trees.as_slice();
    while let Some((head, tail)) = rest.split_first() {
        match (head, tail) {
            (TokenTree::Punct(dollar), [TokenTree::Ident(name), after @ ..])
                if dollar.as_char() == '$' =>
            {
                if name == "crate" {
                    out.push(TokenTree::Ident(Ident::new("crate", name.span())));
                } else {
                    let key = Name::from_string(name.to_string());
                    out.extend(tokens_at(&placeholder_text(kinds.get(&key)), name.span()));
                }
                rest = after;
            }
            (TokenTree::Punct(dollar), [TokenTree::Group(group), after @ ..])
                if dollar.as_char() == '$' && group.delimiter() == Delimiter::Parenthesis =>
            {
                out.extend(replace(group.stream(), kinds));
                rest = after_repetition(after);
            }
            (TokenTree::Group(group), _) => {
                let mut renewed = Group::new(group.delimiter(), replace(group.stream(), kinds));
                renewed.set_span(group.span());
                out.push(TokenTree::Group(renewed));
                rest = tail;
            }
            _ => {
                out.push(head.clone());
                rest = tail;
            }
        }
    }
    out.into_iter().collect()
}

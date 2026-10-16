//! `catch-all-arm`, `refutable-shortcut` and `struct-rest-pattern`, decided from
//! patterns alone.

use alloc::collections::{BTreeMap, BTreeSet};

use proc_macro2::Span;
use syn::{Pat, PatStruct, PatTuple};

use super::paths;
use super::types;
use super::{span_line_col, Scanner};
use crate::index::Kind;
use crate::rules;
use crate::types::{Answer, Col, Count, Holds as _, Line, Message, Name, Text};

/// A pattern or implied catch-all that is analysed for the catch-all rule.
enum Nested<'pat> {
    /// A catch-all implied by an enclosing `..` or catch-all arm, at the given span.
    CatchAll(Span),
    /// A real sub-pattern to classify.
    Pat(&'pat Pat),
}

/// The coarse shape of one pattern alternative.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// A wildcard-like pattern that matches anything.
    CatchAll,
    /// A struct, tuple or slice pattern with sub-patterns, not an enum variant.
    Compound,
    /// Any other pattern, such as a literal, constant or non-variant path.
    Other,
    /// A workspace enum variant.
    Variant,
}

/// A sub-pattern of a constructor, keyed by its position (index or field name).
struct Position<'pat> {
    /// The position within the constructor: an index, a negative index after `..` or a field name.
    key: Text,
    /// The sub-pattern at that position.
    pat: &'pat Pat,
}

/// A `..` that covers the positions from `start` on.
struct Rest {
    /// Span of the `..`.
    span: Span,
    /// First position that the `..` covers.
    start: Count,
}

/// The positions below one constructor pattern.
struct Children<'pat> {
    /// Resolved text naming the constructor, or a marker such as `(tuple)` or `[slice]`.
    constructor: Text,
    /// The sub-patterns of the constructor with their keys.
    positions: Vec<Position<'pat>>,
    /// The `..` markers inside the constructor pattern.
    rests: Vec<Rest>,
}

/// One alternative of a pattern after or-patterns have been split.
struct Alt<'pat> {
    /// Sub-patterns, present when the alternative is a constructor pattern.
    children: Option<Children<'pat>>,
    /// The coarse shape of the alternative.
    shape: Shape,
    /// Span used when the alternative is reported.
    span: Span,
}

/// Which constructor and which position a group of sub-patterns belongs to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    /// The constructor the sub-patterns belong to.
    constructor: Text,
    /// The position under that constructor.
    position: Text,
}

/// Builds an alternative that has no sub-patterns.
const fn simple(shape: Shape, span: Span) -> Alt<'static> {
    Alt {
        children: None,
        shape,
        span,
    }
}

/// Builds the positioned children of a tuple-like pattern, tracking `..` and negative indices.
fn tuple_children<'pat>(
    elems: impl Iterator<Item = &'pat Pat>,
    constructor: Text,
) -> Children<'pat> {
    let all: Vec<&Pat> = elems.collect();
    let rest_at = all.iter().position(|pat| matches!(pat, Pat::Rest(_)));
    let mut positions = Vec::new();
    let mut rests = Vec::new();
    for (index, pat) in all.iter().enumerate() {
        if let Pat::Rest(rest) = pat {
            rests.push(Rest {
                span: rest.dot2_token.spans[0],
                start: Count::new(index),
            });
            continue;
        }
        let after_rest = rest_at.is_some_and(|rest_index| index > rest_index);
        let key = if after_rest {
            let from_end = Count::new(all.len()).minus(Count::new(index));
            match from_end {
                Ok(distance) => format!("-{}", distance.get()),
                Err(_) => format!("-{index}"),
            }
        } else {
            index.to_string()
        };
        positions.push(Position {
            key: Text::from_string(key),
            pat,
        });
    }
    Children {
        constructor,
        positions,
        rests,
    }
}

/// Resolves a bare identifier pattern to its kind.
fn ident_kind(scanner: &Scanner<'_>, ident: &syn::Ident) -> Kind {
    let name = Name::from_string(ident.to_string());
    types::resolve_segments(scanner, &[name]).kind
}

/// Resolves the path of a pattern to its kind.
fn path_kind(scanner: &Scanner<'_>, path: &syn::Path) -> Kind {
    types::resolve_path(scanner, path).kind
}

/// Classifies an identifier pattern, looking through `name @ subpattern`.
fn classify_ident<'pat>(
    scanner: &Scanner<'_>,
    ident: &'pat syn::PatIdent,
    out: &mut Vec<Alt<'pat>>,
) {
    if let Some((_, sub)) = &ident.subpat {
        let before = out.len();
        classify(scanner, sub, out);
        for alt in out.iter_mut().skip(before) {
            if alt.shape == Shape::CatchAll {
                alt.span = ident.ident.span();
            }
        }
        return;
    }
    let name = ident.ident.to_string();
    let shape = match ident_kind(scanner, &ident.ident) {
        Kind::Variant => Shape::Variant,
        Kind::Workspace | Kind::External => Shape::Other,
        Kind::Local | Kind::Unresolved | Kind::Ambiguous => {
            if name.chars().next().is_some_and(char::is_uppercase) {
                Shape::Other
            } else {
                Shape::CatchAll
            }
        }
    };
    out.push(simple(shape, ident.ident.span()));
}

/// Splits a pattern into alternatives and classifies each into `out`.
fn classify<'pat>(scanner: &Scanner<'_>, pat: &'pat Pat, out: &mut Vec<Alt<'pat>>) {
    match pat {
        Pat::Wild(wild) => out.push(simple(Shape::CatchAll, wild.underscore_token.span)),
        Pat::Rest(rest) => out.push(simple(Shape::CatchAll, rest.dot2_token.spans[0])),
        Pat::Ident(ident) => classify_ident(scanner, ident, out),
        Pat::Path(path) => {
            let shape = match path_kind(scanner, &path.path) {
                Kind::Variant => Shape::Variant,
                Kind::Workspace
                | Kind::External
                | Kind::Unresolved
                | Kind::Ambiguous
                | Kind::Local => Shape::Other,
            };
            out.push(simple(shape, paths::first_span(&path.path)));
        }
        Pat::TupleStruct(tuple_struct) => {
            let shape = match path_kind(scanner, &tuple_struct.path) {
                Kind::Variant => Shape::Variant,
                Kind::Workspace
                | Kind::External
                | Kind::Unresolved
                | Kind::Ambiguous
                | Kind::Local => Shape::Compound,
            };
            let key = types::resolve_path(scanner, &tuple_struct.path).text();
            out.push(Alt {
                children: Some(tuple_children(tuple_struct.elems.iter(), key)),
                shape,
                span: paths::first_span(&tuple_struct.path),
            });
        }
        Pat::Struct(pat_struct) => classify_struct(scanner, pat_struct, out),
        Pat::Tuple(tuple) => classify_tuple(tuple, out),
        Pat::Slice(slice) => out.push(Alt {
            children: Some(tuple_children(slice.elems.iter(), Text::of("[slice]"))),
            shape: Shape::Compound,
            span: slice.bracket_token.span.join(),
        }),
        Pat::Or(or) => {
            for case in &or.cases {
                classify(scanner, case, out);
            }
        }
        Pat::Paren(paren) => classify(scanner, &paren.pat, out),
        Pat::Reference(reference) => classify(scanner, &reference.pat, out),
        Pat::Type(typed) => classify(scanner, &typed.pat, out),
        _ => out.push(simple(Shape::Other, Span::call_site())),
    }
}

/// Classifies a tuple pattern as a compound alternative with positional children.
fn classify_tuple<'pat>(tuple: &'pat PatTuple, out: &mut Vec<Alt<'pat>>) {
    out.push(Alt {
        children: Some(tuple_children(tuple.elems.iter(), Text::of("(tuple)"))),
        shape: Shape::Compound,
        span: tuple.paren_token.span.join(),
    });
}

/// Classifies a struct pattern as a compound or variant alternative with field children.
fn classify_struct<'pat>(
    scanner: &Scanner<'_>,
    pat_struct: &'pat PatStruct,
    out: &mut Vec<Alt<'pat>>,
) {
    let shape = match path_kind(scanner, &pat_struct.path) {
        Kind::Variant => Shape::Variant,
        Kind::Workspace | Kind::External | Kind::Unresolved | Kind::Ambiguous | Kind::Local => {
            Shape::Compound
        }
    };
    let constructor = types::resolve_path(scanner, &pat_struct.path).text();
    let positions = pat_struct
        .fields
        .iter()
        .map(|field| {
            let name = match &field.member {
                syn::Member::Named(ident) => ident.to_string(),
                syn::Member::Unnamed(index) => index.index.to_string(),
            };
            Position {
                key: Text::from_string(name),
                pat: field.pat.as_ref(),
            }
        })
        .collect();
    let rests = match &pat_struct.rest {
        Some(rest) => vec![Rest {
            span: rest.dot2_token.spans[0],
            start: Count::new(0),
        }],
        None => Vec::new(),
    };
    out.push(Alt {
        children: Some(Children {
            constructor,
            positions,
            rests,
        }),
        shape,
        span: paths::first_span(&pat_struct.path),
    });
}

/// Reports catch-all patterns that sit beside workspace variants, recursing into sub-patterns.
///
/// A catch-all at one level is pushed into the group of every nested position
/// under that level. In `match (a, b) { (K::A, K::A) => .., _ => .. }` the `_`
/// is a catch-all for position 0 and for position 1, and is reported when either
/// position names workspace variants elsewhere in the match. This is stricter
/// than a catch-all beside variants at one position, deliberately: a whole-tuple
/// `_` covers each nested position. A `..` rest pattern in a tuple, slice or
/// struct pattern is treated the same way for the positions it covers.
fn analyze(
    scanner: &mut Scanner<'_>,
    entries: &[Nested<'_>],
    reported: &mut BTreeSet<(Line, Col)>,
) {
    let mut alts: Vec<Alt<'_>> = Vec::new();
    for entry in entries {
        match entry {
            Nested::CatchAll(span) => alts.push(simple(Shape::CatchAll, *span)),
            Nested::Pat(pat) => classify(scanner, pat, &mut alts),
        }
    }
    let has_variant = alts.iter().any(|alt| alt.shape == Shape::Variant);
    let catch_alls: Vec<Span> = alts
        .iter()
        .filter(|alt| alt.shape == Shape::CatchAll)
        .map(|alt| alt.span)
        .collect();
    if has_variant {
        for span in &catch_alls {
            if reported.insert(span_line_col(*span)) {
                scanner.report(
                    *span,
                    rules::CATCH_ALL_ARM,
                    &Message::of(
                        "catch-all pattern beside workspace enum variants; name every variant",
                    ),
                );
            }
        }
    }
    for group in nested_groups(&alts, &catch_alls) {
        analyze(scanner, &group, reported);
    }
}

/// The sub-pattern groups below `alts`, with implied catch-alls added.
fn nested_groups<'pat>(alts: &[Alt<'pat>], catch_alls: &[Span]) -> Vec<Vec<Nested<'pat>>> {
    let mut groups: BTreeMap<GroupKey, Vec<Nested<'pat>>> = BTreeMap::new();
    let mut rests: BTreeMap<Text, Vec<&Rest>> = BTreeMap::new();
    for alt in alts {
        let Some(children) = &alt.children else {
            continue;
        };
        for position in &children.positions {
            let key = GroupKey {
                constructor: children.constructor.clone(),
                position: position.key.clone(),
            };
            groups
                .entry(key)
                .or_default()
                .push(Nested::Pat(position.pat));
        }
        rests
            .entry(children.constructor.clone())
            .or_default()
            .extend(children.rests.iter());
    }
    for (key, entries) in &mut groups {
        let index = key.position.as_str().parse::<usize>();
        if let Some(rest_list) = rests.get(&key.constructor) {
            for rest in rest_list {
                let covers = match &index {
                    Ok(found) => *found >= rest.start.get(),
                    Err(_) => true,
                };
                if covers {
                    entries.push(Nested::CatchAll(rest.span));
                }
            }
        }
        for span in catch_alls {
            entries.push(Nested::CatchAll(*span));
        }
    }
    groups.into_values().collect()
}

/// `catch-all-arm` over the arms of one `match`.
pub(crate) fn check_match_arms(scanner: &mut Scanner<'_>, arms: &[syn::Arm]) {
    let entries: Vec<Nested<'_>> = arms.iter().map(|arm| Nested::Pat(&arm.pat)).collect();
    let mut reported = BTreeSet::new();
    analyze(scanner, &entries, &mut reported);
}

/// Does any of `pats` name a workspace enum variant anywhere?
fn any_names<'pat, Pats>(scanner: &Scanner<'_>, mut pats: Pats) -> Answer
where
    Pats: Iterator<Item = &'pat Pat>,
{
    pats.any(|inner| pat_names_variant(scanner, inner) == Answer::Yes)
        .answer()
}

/// Does the pattern name a workspace enum variant anywhere?
pub(crate) fn pat_names_variant(scanner: &Scanner<'_>, pat: &Pat) -> Answer {
    match pat {
        Pat::Ident(ident) => {
            let own = match ident_kind(scanner, &ident.ident) {
                Kind::Variant => Answer::Yes,
                Kind::Workspace
                | Kind::External
                | Kind::Unresolved
                | Kind::Ambiguous
                | Kind::Local => Answer::No,
            };
            match (own, &ident.subpat) {
                (Answer::Yes, _) => Answer::Yes,
                (Answer::No, Some((_, sub))) => pat_names_variant(scanner, sub),
                (Answer::No, None) => Answer::No,
            }
        }
        Pat::Path(path) => path_is_variant(scanner, &path.path),
        Pat::TupleStruct(tuple_struct) => match path_is_variant(scanner, &tuple_struct.path) {
            Answer::Yes => Answer::Yes,
            Answer::No => any_names(scanner, tuple_struct.elems.iter()),
        },
        Pat::Struct(pat_struct) => match path_is_variant(scanner, &pat_struct.path) {
            Answer::Yes => Answer::Yes,
            Answer::No => any_names(scanner, pat_struct.fields.iter().map(|field| &*field.pat)),
        },
        Pat::Tuple(tuple) => any_names(scanner, tuple.elems.iter()),
        Pat::Slice(slice) => any_names(scanner, slice.elems.iter()),
        Pat::Or(or) => any_names(scanner, or.cases.iter()),
        Pat::Paren(paren) => pat_names_variant(scanner, &paren.pat),
        Pat::Reference(reference) => pat_names_variant(scanner, &reference.pat),
        Pat::Type(typed) => pat_names_variant(scanner, &typed.pat),
        _ => Answer::No,
    }
}

/// Does the path name a workspace enum variant?
fn path_is_variant(scanner: &Scanner<'_>, path: &syn::Path) -> Answer {
    match path_kind(scanner, path) {
        Kind::Variant => Answer::Yes,
        Kind::Workspace | Kind::External | Kind::Unresolved | Kind::Ambiguous | Kind::Local => {
            Answer::No
        }
    }
}

/// Flags a refutable pattern that names a workspace enum variant outside a `match`.
pub(crate) fn check_refutable(scanner: &mut Scanner<'_>, pat: &Pat, span: Span) {
    if pat_names_variant(scanner, pat) == Answer::Yes {
        let mut rests = Vec::new();
        variant_rests(scanner, pat, &mut rests);
        for rest in rests {
            let (line, col) = span_line_col(rest);
            let place = (scanner.path.clone(), line, col);
            scanner.refutable_rests.push(place);
        }
        scanner.report(
            span,
            rules::REFUTABLE_SHORTCUT,
            &Message::of(
                "pattern names a workspace enum variant outside a `match`; write a `match` naming every variant",
            ),
        );
    }
}

/// Collects the spans of the `..` of every workspace enum-variant pattern
/// inside `pat` (a `..` of a plain struct pattern is not one of them).
fn variant_rests(scanner: &Scanner<'_>, pat: &Pat, out: &mut Vec<Span>) {
    match pat {
        Pat::Ident(ident) => {
            if let Some((_, sub)) = &ident.subpat {
                variant_rests(scanner, sub, out);
            }
        }
        Pat::Struct(pat_struct) => {
            if path_is_variant(scanner, &pat_struct.path) == Answer::Yes {
                if let Some(rest) = &pat_struct.rest {
                    out.push(rest.dot2_token.spans[0]);
                }
            }
            for field in &pat_struct.fields {
                variant_rests(scanner, &field.pat, out);
            }
        }
        Pat::TupleStruct(tuple_struct) => {
            let variant = path_is_variant(scanner, &tuple_struct.path) == Answer::Yes;
            for elem in &tuple_struct.elems {
                match elem {
                    Pat::Rest(rest) if variant => out.push(rest.dot2_token.spans[0]),
                    _ => variant_rests(scanner, elem, out),
                }
            }
        }
        Pat::Tuple(tuple) => {
            for elem in &tuple.elems {
                variant_rests(scanner, elem, out);
            }
        }
        Pat::Slice(slice) => {
            for elem in &slice.elems {
                variant_rests(scanner, elem, out);
            }
        }
        Pat::Or(or) => {
            for case in &or.cases {
                variant_rests(scanner, case, out);
            }
        }
        Pat::Paren(paren) => variant_rests(scanner, &paren.pat, out),
        Pat::Reference(reference) => variant_rests(scanner, &reference.pat, out),
        Pat::Type(typed) => variant_rests(scanner, &typed.pat, out),
        _ => {}
    }
}

/// Flags the `..` of a struct pattern whose type is declared in the workspace.
pub(crate) fn check_struct_rest(scanner: &mut Scanner<'_>, pat: &PatStruct) {
    if let Some(rest) = &pat.rest {
        report_rest(scanner, &pat.path, rest.dot2_token.spans[0]);
    }
}

/// Flags the `..` of a tuple-struct or tuple-variant pattern whose type is
/// declared in the workspace.
pub(crate) fn check_tuple_struct_rest(scanner: &mut Scanner<'_>, pat: &syn::PatTupleStruct) {
    for elem in &pat.elems {
        if let Pat::Rest(rest) = elem {
            report_rest(scanner, &pat.path, rest.dot2_token.spans[0]);
        }
    }
}

/// Has `catch-all-arm` already reported a finding at `span` in this file?
fn reported_by_catch_all(scanner: &Scanner<'_>, span: Span) -> Answer {
    let rule = Name::of(rules::CATCH_ALL_ARM.name());
    let (line, col) = span_line_col(span);
    scanner
        .findings
        .iter()
        .any(|found| {
            found.rule == rule
                && found.path == scanner.path
                && found.line == line
                && found.col == col
        })
        .answer()
}

/// Is the `..` at `span` part of a pattern `refutable-shortcut` reports?
fn owned_by_refutable(scanner: &Scanner<'_>, span: Span) -> Answer {
    let (line, col) = span_line_col(span);
    scanner
        .refutable_rests
        .iter()
        .any(|(path, at_line, at_col)| *path == scanner.path && *at_line == line && *at_col == col)
        .answer()
}

/// Reports the `..` at `span` when `path` names a workspace struct or variant
/// and neither `catch-all-arm` nor `refutable-shortcut` already reports that
/// `..` (or the pattern it is in).
fn report_rest(scanner: &mut Scanner<'_>, path: &syn::Path, span: Span) {
    let resolved = types::resolve_path(scanner, path);
    let shown_segments = match resolved.kind {
        Kind::Variant => 2,
        Kind::Workspace | Kind::Local => 1,
        Kind::Ambiguous | Kind::External | Kind::Unresolved => return,
    };
    if scanner.env.resolver.is_fieldless(&resolved) == Answer::Yes {
        return;
    }
    if reported_by_catch_all(scanner, span) == Answer::Yes
        || owned_by_refutable(scanner, span) == Answer::Yes
    {
        return;
    }
    let mut tail: Vec<&str> = resolved
        .segments
        .iter()
        .rev()
        .take(shown_segments)
        .map(Name::as_str)
        .collect();
    tail.reverse();
    let shown = tail.join("::");
    scanner.report(
        span,
        rules::STRUCT_REST_PATTERN,
        &Message::from_string(format!(
            "`..` ignores fields of workspace type `{shown}`, so a new field is silently skipped; list every field (bind an ignored field to an underscore-prefixed name such as `a: _a`; `a: _` is fine when another field is bound)"
        )),
    );
}

/// Is `path` the std `Err` (`Err`, `Result::Err`, `std::result::Result::Err`),
/// not a workspace item of that name? `leaf` is `Err` or `Ok`.
#[must_use]
fn is_std_variant(scanner: &Scanner<'_>, path: &syn::Path, leaf: &Name) -> Answer {
    if path
        .segments
        .last()
        .is_none_or(|last| last.ident != leaf.as_str())
    {
        return Answer::No;
    }
    let resolved = types::resolve_path(scanner, path);
    match resolved.kind {
        Kind::Local | Kind::Variant | Kind::Workspace => Answer::No,
        Kind::Ambiguous | Kind::External | Kind::Unresolved => {
            let text = resolved.text();
            let own = leaf.as_str().to_owned();
            let spelled = format!("Result::{}", leaf.as_str());
            let full = format!("std::result::Result::{}", leaf.as_str());
            (text.as_str() == own || text.as_str() == spelled || text.as_str() == full).answer()
        }
    }
}

/// Is `expr` an empty block, `()` or `{ () }`?
#[must_use]
fn is_empty_body(expr: &syn::Expr) -> Answer {
    match expr {
        syn::Expr::Tuple(tuple) => tuple.elems.is_empty().answer(),
        syn::Expr::Block(block) => match block.block.stmts.as_slice() {
            [] => Answer::Yes,
            [syn::Stmt::Expr(inner, None)] => {
                matches!(inner, syn::Expr::Tuple(tuple) if tuple.elems.is_empty()).answer()
            }
            _ => Answer::No,
        },
        _ => Answer::No,
    }
}

/// The span of the `_`, `..` or `_name` inside an `Err(..)` pattern that ignores
/// the error, with the span of the pattern itself.
#[must_use]
fn ignored_error(scanner: &Scanner<'_>, pat: &Pat) -> Option<(Span, Span)> {
    match pat {
        Pat::TupleStruct(tuple_struct)
            if is_std_variant(scanner, &tuple_struct.path, &Name::of("Err")) == Answer::Yes =>
        {
            let [inner] = tuple_struct.elems.iter().collect::<Vec<_>>()[..] else {
                return None;
            };
            let at = match inner {
                Pat::Wild(wild) => wild.underscore_token.span,
                Pat::Rest(rest) => rest.dot2_token.spans[0],
                Pat::Ident(ident)
                    if ident.subpat.is_none() && ident.ident.to_string().starts_with('_') =>
                {
                    ident.ident.span()
                }
                _ => return None,
            };
            Some((paths::first_span(&tuple_struct.path), at))
        }
        Pat::Or(or) => or
            .cases
            .iter()
            .find_map(|case| ignored_error(scanner, case)),
        Pat::Paren(paren) => ignored_error(scanner, &paren.pat),
        _ => None,
    }
}

/// `swallowed-error` over the arms of one `match`: an `Err` arm that ignores the
/// error and does nothing. A `_` that `catch-all-arm` already reported is left to it.
pub(crate) fn check_swallowed_arms(scanner: &mut Scanner<'_>, arms: &[syn::Arm]) {
    for arm in arms {
        if is_empty_body(&arm.body) == Answer::No {
            continue;
        }
        let Some((span, inner)) = ignored_error(scanner, &arm.pat) else {
            continue;
        };
        if reported_by_catch_all(scanner, inner) == Answer::Yes {
            continue;
        }
        scanner.report(
            span,
            rules::SWALLOWED_ERROR,
            &Message::of(
                "an empty `Err` arm drops the error silently; handle the error or return it",
            ),
        );
    }
}

/// Every `let` of a condition: the condition itself when it is one, and each
/// operand of an `&&` chain (a let chain, `if let Ok(v) = f() && ready`).
fn let_operands<'expr>(cond: &'expr syn::Expr, out: &mut Vec<&'expr syn::ExprLet>) {
    match cond {
        syn::Expr::Let(expr_let) => out.push(expr_let),
        syn::Expr::Binary(binary) if matches!(binary.op, syn::BinOp::And(_)) => {
            let_operands(&binary.left, out);
            let_operands(&binary.right, out);
        }
        syn::Expr::Paren(paren) => let_operands(&paren.expr, out),
        _ => {}
    }
}

/// `swallowed-error` for `if let Ok(..) = ..` and `while let Ok(..) = ..` with no
/// `else`, also when the `let` is an operand of an `&&` chain. A pattern that
/// names a workspace variant belongs to `refutable-shortcut`.
pub(crate) fn check_swallowed_let(scanner: &mut Scanner<'_>, cond: &syn::Expr) {
    let mut lets = Vec::new();
    let_operands(cond, &mut lets);
    for expr_let in lets {
        let Pat::TupleStruct(tuple_struct) = expr_let.pat.as_ref() else {
            continue;
        };
        if is_std_variant(scanner, &tuple_struct.path, &Name::of("Ok")) == Answer::No
            || pat_names_variant(scanner, &expr_let.pat) == Answer::Yes
        {
            continue;
        }
        scanner.report(
            paths::first_span(&tuple_struct.path),
            rules::SWALLOWED_ERROR,
            &Message::of("`let Ok(..)` without an `else` drops the error silently; match both cases and handle the `Err`"),
        );
    }
}

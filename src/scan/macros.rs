//! Macro invocations: name rules, comment requirements, `include!`, and
//! body parsing (items, then statements, then comma-separated expressions).

use core::mem;
use proc_macro2::{Span, TokenStream};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::visit::Visit as _;
use syn::{Expr, Pat, Token};

use super::exprs;
use super::items;
use super::paths;
use super::patterns;
use super::substitute;
use super::tokens;
use super::{AssertionSeen, Scanner};
use crate::rules;
use crate::types::{Answer, Col, Count, Line, Message, Name, Text};

/// Whether the config names a macro in `[[macro-comment]]`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Documented {
    /// The macro has no comment requirement.
    No,
    /// The macro has a comment requirement.
    Yes,
}

/// A macro call whose tokens are scanned as a body.
struct BodySite<'mac> {
    /// Whether the config names the macro in `[[macro-comment]]`.
    documented: Documented,
    /// The macro's name.
    leaf: Name,
    /// The call.
    mac: &'mac syn::Macro,
    /// Where findings about the call itself sit.
    span: Span,
}

/// A comment a macro invocation must carry above it.
struct Requirement {
    /// Comment prefix that must appear above the macro call.
    prefix: Text,
    /// How many lines above the macro call the comment may sit.
    within: Count,
}

/// The parsed arguments of a `matches!`-style macro call.
struct MatchesArgs {
    /// The expression being matched.
    expr: Expr,
    /// The optional `if` guard following the pattern.
    guard: Option<Expr>,
    /// The pattern the expression is matched against.
    pat: Pat,
}

impl Parse for MatchesArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let expr: Expr = input.parse()?;
        input.parse::<Token![,]>().map(|_token| ())?;
        let pat = Pat::parse_multi_with_leading_vert(input)?;
        let guard = if input.peek(Token![if]) {
            input.parse::<Token![if]>().map(|_token| ())?;
            Some(input.parse::<Expr>()?)
        } else {
            None
        };
        if input.peek(Token![,]) {
            input.parse::<Token![,]>().map(|_token| ())?;
        }
        Ok(Self { expr, guard, pat })
    }
}

/// Applies the macro rules to one invocation and scans its body.
pub(crate) fn on_macro(scanner: &mut Scanner<'_>, mac: &syn::Macro) {
    paths::check_macro_name(scanner, &mac.path);
    let Some(last) = mac.path.segments.last() else {
        return;
    };
    let leaf = last.ident.to_string();
    let span = paths::first_span(&mac.path);
    let is_assertion = rules::TEST_ASSERTION_MACROS.contains(&leaf.as_str())
        || scanner
            .env
            .config
            .assertion_macros
            .iter()
            .any(|name| name.as_str() == leaf);
    if is_assertion {
        if let Some(slot) = scanner.assertions.last_mut() {
            *slot = AssertionSeen::Yes;
        }
    }
    let documented = check_macro_comment(scanner, &Name::from_string(leaf.clone()), span);
    if leaf == "include" {
        on_include(scanner, mac, span);
        return;
    }
    let site = BodySite {
        documented,
        leaf: Name::from_string(leaf.clone()),
        mac,
        span,
    };
    if leaf == "matches" || leaf == "assert_matches" || leaf == "debug_assert_matches" {
        match syn::parse2::<MatchesArgs>(mac.tokens.clone()) {
            Ok(args) => scan_matches(scanner, &args, span),
            Err(_unmatched) => scan_as_body(scanner, &site),
        }
    } else {
        scan_as_body(scanner, &site);
    }
}

/// Scans the parsed arguments of a `matches!`-style call: the pattern is
/// checked as a refutable shortcut and every part is visited.
fn scan_matches(scanner: &mut Scanner<'_>, args: &MatchesArgs, span: Span) {
    patterns::check_refutable(scanner, &args.pat, span);
    scanner.visit_expr(&args.expr);
    scanner.visit_pat(&args.pat);
    if let Some(guard) = &args.guard {
        scanner.visit_expr(guard);
    }
}

/// Scans a macro call's tokens as a body under the macro's own item label.
fn scan_as_body(scanner: &mut Scanner<'_>, site: &BodySite<'_>) {
    scanner.push_item(Text::from_string(format!("macro {}!", site.leaf.as_str())));
    match site.documented {
        Documented::Yes => {
            // A macro the config names (`[[macro-comment]]`, such as `proptest!`) is
            // accounted for by its comment requirement, so its body is not
            // reported as unparsed.
            if parse_and_scan(scanner, &site.mac.tokens) == Answer::No {
                tokens::scan_tokens(scanner, site.mac.tokens.clone());
            }
        }
        Documented::No => scan_body(scanner, site.mac.tokens.clone(), site.span),
    }
    scanner.pop_item();
}

/// Tries items, statements, then comma-separated expressions; falls back
/// to the token-level scan, and reports `unparsed-macro`, when none parse.
pub(crate) fn scan_body(scanner: &mut Scanner<'_>, tokens: TokenStream, span: Span) {
    if parse_and_scan(scanner, &tokens) == Answer::No {
        tokens::scan_unparsed(scanner, tokens, span);
    }
}

/// Parses `tokens` as items, statements, then comma-separated expressions and
/// runs every rule on the first parse that works. `No` when none parses.
fn parse_and_scan(scanner: &mut Scanner<'_>, tokens: &TokenStream) -> Answer {
    match syn::parse2::<syn::File>(tokens.clone()) {
        Ok(file) => {
            scanner.visit_file(&file);
            Answer::Yes
        }
        Err(_not_items) => parse_as_statements(scanner, tokens),
    }
}

/// The second attempt of `parse_and_scan`: statements; expressions when they do not parse.
fn parse_as_statements(scanner: &mut Scanner<'_>, tokens: &TokenStream) -> Answer {
    match Parser::parse2(syn::Block::parse_within, tokens.clone()) {
        Ok(stmts) => {
            scanner.blocks.push(items::block_scope(&stmts));
            exprs::check_underscore_bindings(scanner, &stmts);
            for stmt in &stmts {
                scanner.visit_stmt(stmt);
            }
            match scanner.blocks.pop() {
                Some(_) | None => {}
            }
            Answer::Yes
        }
        Err(_not_statements) => parse_as_expressions(scanner, tokens),
    }
}

/// The last attempt of `parse_and_scan`: comma-separated expressions; `No` when they do not parse.
fn parse_as_expressions(scanner: &mut Scanner<'_>, tokens: &TokenStream) -> Answer {
    let parsed: Result<Punctuated<Expr, Token![,]>, _> =
        Parser::parse2(Punctuated::parse_terminated, tokens.clone());
    match parsed {
        Ok(exprs) => {
            for expr in &exprs {
                scanner.visit_expr(expr);
            }
            Answer::Yes
        }
        Err(_not_expressions) => Answer::No,
    }
}

/// Scans a `macro_rules!` definition. Each rule's transcriber is parsed with the
/// metavariables replaced by placeholders (`$x:ident` by an identifier, `$t:ty`
/// by a type, `$( .. )*` by one copy of its body), so every rule runs on it as on
/// ordinary code. The expansions themselves are never scanned, so a finding
/// inside a definition, or a transcriber that does not parse even then, is also
/// reported as `unparsed-macro`: the baseline records the gap.
pub(crate) fn scan_macro_rules(scanner: &mut Scanner<'_>, tokens: TokenStream, span: Span) {
    let before = scanner.findings.len();
    let Some(transcribers) = substitute::transcribers(tokens.clone()) else {
        tokens::scan_unparsed(scanner, tokens, span);
        return;
    };
    let mut unparsed = Answer::No;
    for transcriber in transcribers {
        if parse_and_scan(scanner, &transcriber) == Answer::No {
            tokens::scan_tokens(scanner, transcriber);
            unparsed = Answer::Yes;
        }
    }
    if unparsed == Answer::Yes || scanner.findings.len() > before {
        scanner.report(
            span,
            rules::UNPARSED_MACRO,
            &Message::of(
                "macro_rules! body is scanned as a template, not as expansions; the findings, or the unparsed rule, are recorded here",
            ),
        );
    }
}

/// Reports a macro call that lacks a comment required by the config, and says
/// whether the config names the macro at all.
fn check_macro_comment(scanner: &mut Scanner<'_>, leaf_name: &Name, span: Span) -> Documented {
    let leaf = leaf_name.as_str();
    let line = Count::new(span.start().line);
    let wanted: Vec<Requirement> = scanner
        .env
        .config
        .macro_comments
        .iter()
        .filter(|entry| {
            let name = entry.macro_name.as_str();
            name == leaf || name.rsplit("::").next() == Some(leaf)
        })
        .map(|entry| Requirement {
            prefix: entry.prefix.clone(),
            within: entry.within_lines,
        })
        .collect();
    let documented = if wanted.is_empty() {
        Documented::No
    } else {
        Documented::Yes
    };
    for requirement in wanted {
        let lines: Vec<&str> = scanner.source.as_str().lines().collect();
        let first = match line.minus(requirement.within) {
            Ok(found) if found.get() >= 1 => found,
            Ok(_) | Err(_) => Count::new(1),
        };
        let found = (first.get()..line.get()).any(|number| match Count::new(number).prev() {
            Ok(index) => lines
                .get(index.get())
                .is_some_and(|text| text.trim().starts_with(requirement.prefix.as_str())),
            Err(_) => false,
        });
        if !found {
            scanner.report(
                span,
                rules::MACRO_NEEDS_COMMENT,
                &Message::from_string(format!(
                    "`{leaf}!` needs a `{}` comment within {} lines above",
                    requirement.prefix.as_str().trim(),
                    requirement.within.get()
                )),
            );
        }
    }
    documented
}

/// Scans the file named by an `include!` call as if it were inlined at the call.
///
/// A string-literal path is read relative to the including file. An `include!`
/// inside an included file is reported as `unscannable-include` ("nested
/// include! is not followed", `include_depth` is not zero) rather than followed.
/// A non-literal argument such as `include!(concat!(env!("OUT_DIR"), ..))` is
/// reported too. The in-memory canary loader reports only the non-literal form;
/// it has no filesystem, so literal includes in a canary are ignored there.
fn on_include(scanner: &mut Scanner<'_>, mac: &syn::Macro, span: Span) {
    let start = span.start();
    let key = (Line::new(start.line), Col::new(start.column));
    let found = if scanner.include_depth.get() == 0 {
        scanner
            .file
            .includes
            .iter()
            .find(|inc| (inc.line, inc.col) == key)
    } else {
        None
    };
    let Some(included) = found else {
        if syn::parse2::<syn::LitStr>(mac.tokens.clone()).is_ok() {
            scanner.report(
                span,
                rules::UNSCANNABLE_INCLUDE,
                &Message::of("nested include! is not followed"),
            );
        }
        return;
    };
    let Ok(tokens) = included.source.as_str().parse::<TokenStream>() else {
        scanner.report(
            span,
            rules::UNSCANNABLE_INCLUDE,
            &Message::of("included file cannot be tokenised"),
        );
        return;
    };
    let saved_depth = scanner.include_depth;
    let Ok(deeper) = saved_depth.next() else {
        scanner.report(
            span,
            rules::UNSCANNABLE_INCLUDE,
            &Message::of("include! is nested too deeply to follow"),
        );
        return;
    };
    let saved_path = mem::replace(&mut scanner.path, included.path.clone());
    let saved_source = mem::replace(&mut scanner.source, &included.source);
    scanner.include_depth = deeper;
    scan_body(scanner, tokens, span);
    scanner.include_depth = saved_depth;
    scanner.source = saved_source;
    scanner.path = saved_path;
}

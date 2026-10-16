//! Path, method-name and macro-name rules.
use proc_macro2::Span;

use super::types;

use super::Scanner;

use crate::config::Role;

use crate::index::{Kind, PathPattern, Resolved};

use crate::loader::Scope;

use crate::rules::{self, PatternTable, RuleId};

use crate::types::{Answer, Holds as _, Message, Name, Text};

/// `Path` methods that probe the file system: ambient input like `std::fs`.
const PROBE_METHODS: &[&str] = &[
    "canonicalize",
    "exists",
    "is_dir",
    "is_file",
    "is_symlink",
    "metadata",
    "read_dir",
    "read_link",
    "symlink_metadata",
    "try_exists",
];

/// The probe names that few other types have, so the call is flagged by name.
/// The others (`exists`, `metadata`, `is_file`, `is_dir`) are flagged only on a
/// receiver that is visibly a `Path` or `PathBuf`.
const PROBE_NAMES: &[&str] = &[
    "canonicalize",
    "is_symlink",
    "read_dir",
    "read_link",
    "symlink_metadata",
    "try_exists",
];

/// Methods that keep a `Path` receiver a path when chained.
const PATH_CHAIN: &[&str] = &[
    "as_path",
    "clone",
    "join",
    "parent",
    "to_owned",
    "to_path_buf",
    "with_extension",
    "with_file_name",
];

/// Message of every `memory-leak` finding.
const LEAK_MESSAGE: &str =
    "leaked memory is never reclaimed and hides who owns it; restructure so the owner drops it";

/// The syntactic position in which a path is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PathRole {
    /// Inside an attribute.
    Attr,
    /// As the callee of a call expression.
    Callee,
    /// In an expression other than a callee.
    Expr,
    /// As the name of a macro invocation.
    Macro,
    /// In a pattern.
    Pat,
    /// In a type.
    Type,
    /// In a `use` declaration.
    Use,
}

/// Whether a path carries a `<T as Trait>` self type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Qself {
    /// No qualified self type.
    Absent,
    /// A `<T as Trait>` prefix.
    Present,
}

/// A rule hit before it is reported.
struct Hit {
    /// Text describing why the path is flagged.
    message: Message,
    /// Name of the rule that fired.
    rule: Name,
}

/// Reports whether a path has a qualified self type, given the optional `qself`.
#[must_use]
pub(crate) const fn qself_of<T>(found: Option<&T>) -> Qself {
    match found {
        Some(_) => Qself::Present,
        None => Qself::Absent,
    }
}

/// Builds a `Hit` for a built-in rule.
fn hit(rule: RuleId, message: &Message) -> Hit {
    Hit {
        message: message.clone(),
        rule: Name::of(rule.name()),
    }
}

/// Builds the rule name used for findings from the ban set `id`.
fn banned_name(id: &Name) -> Name {
    Name::from_string(format!("{}:{}", rules::BANNED.name(), id.as_str()))
}

/// Returns the span of the first path segment, or the call site when the path is empty.
pub(crate) fn first_span(path: &syn::Path) -> Span {
    match path.segments.first() {
        Some(first) => first.ident.span(),
        None => Span::call_site(),
    }
}

/// Resolves `path` and applies every path rule to it.
pub(crate) fn check_path(
    scanner: &mut Scanner<'_>,
    path: &syn::Path,
    qself: Qself,
    role: PathRole,
) {
    let span = first_span(path);
    let resolved = types::resolve_path(scanner, path);
    check_resolved(scanner, &resolved, span, role);
    let path_form = path.segments.len() >= 2 || qself == Qself::Present;
    let in_expression = match role {
        PathRole::Expr | PathRole::Callee => true,
        PathRole::Type | PathRole::Pat | PathRole::Attr | PathRole::Use | PathRole::Macro => false,
    };
    if path_form && in_expression {
        if let Some(last) = path.segments.last() {
            let name = Name::from_string(last.ident.to_string());
            check_method_name(scanner, last.ident.span(), &name);
        }
    }
}

/// Applies every path rule to an already resolved path.
pub(crate) fn check_resolved(
    scanner: &mut Scanner<'_>,
    resolved: &Resolved,
    span: Span,
    role: PathRole,
) {
    check_float(scanner, resolved, span);
    match resolved.kind {
        Kind::Ambiguous => check_ambiguous(scanner, resolved, span, role),
        Kind::Local => {}
        Kind::Variant | Kind::Workspace | Kind::External | Kind::Unresolved => {
            let text = resolved.text();
            for found in path_hits(scanner, resolved) {
                let subject = Message::from_string(format!("reference to `{}`", text.as_str()));
                emit_hit(scanner, span, &found, &subject);
            }
        }
    }
}

/// Flags every path with an `f32` or `f64` segment (types, constants such as
/// `f64::NAN` or `std::f64::consts::PI`, and associated functions such as
/// `f64::from`), in every position, unless the path names a workspace item.
fn check_float(scanner: &mut Scanner<'_>, resolved: &Resolved, span: Span) {
    match resolved.kind {
        Kind::Local | Kind::Variant | Kind::Workspace => return,
        Kind::Ambiguous | Kind::External | Kind::Unresolved => {}
    }
    let names_float = resolved
        .segments
        .iter()
        .any(|segment| matches!(segment.as_str(), "f32" | "f64"));
    if names_float {
        scanner.report(
            span,
            rules::FLOAT_TYPE,
            &Message::from_string(format!("float type or item `{}`", resolved.text().as_str())),
        );
    }
}

/// Reports a hit, prefixing its message with the subject.
fn emit_hit(scanner: &mut Scanner<'_>, span: Span, found: &Hit, subject: &Message) {
    let message = Message::from_string(format!("{}: {}", subject.as_str(), found.message.as_str()));
    scanner.report_named(span, &found.rule, &message);
}

/// Hits against the built-in path tables.
fn table_hits(scanner: &Scanner<'_>, resolved: &Resolved, hits: &mut Vec<Hit>) {
    if resolved.is_in_table(rules::UNORDERED_PATHS) == Answer::Yes {
        hits.push(hit(
            rules::UNORDERED_COLLECTION,
            &Message::of("iteration order is not deterministic; use BTreeMap/BTreeSet"),
        ));
    }
    if resolved.is_in_table(rules::AMBIENT_PATHS) == Answer::Yes
        && ambient_allowed(scanner, &fs_equivalent(resolved)) == Answer::No
    {
        hits.push(hit(
            rules::AMBIENT_INPUT,
            &Message::of("ambient input or effect is not allowed here"),
        ));
    }
    if resolved.is_in_table(rules::PANIC_PATHS) == Answer::Yes {
        hits.push(hit(
            rules::ABORT_PANIC,
            &Message::of("aborts the process; return an error"),
        ));
    }
    if resolved.is_in_table(rules::MODULAR_TYPE_PATHS) == Answer::Yes {
        hits.push(hit(
            rules::MODULAR_ARITH,
            &Message::of("modular arithmetic hides overflow; use checked operations"),
        ));
    }
    if resolved.is_in_table(rules::LEAK_PATHS) == Answer::Yes {
        hits.push(hit(rules::MEMORY_LEAK, &Message::of(LEAK_MESSAGE)));
    }
}

/// A `Path`/`PathBuf` probe (`std::path::Path::exists`) reads the file system, so
/// it is allowed exactly where `std::fs` is; every other path stands for itself.
fn fs_equivalent(resolved: &Resolved) -> Resolved {
    let names: Vec<&str> = resolved.segments.iter().map(Name::as_str).collect();
    match names.as_slice() {
        ["std", "path", "Path" | "PathBuf", probe] if PROBE_METHODS.contains(probe) => {
            synthetic(&Text::of("std::fs"))
        }
        _ => resolved.clone(),
    }
}

/// Hits against the config's ban sets.
fn ban_hits(scanner: &Scanner<'_>, resolved: &Resolved, hits: &mut Vec<Hit>) {
    for ban in &scanner.env.config.bans {
        if !ban
            .files
            .iter()
            .any(|glob| glob.matches(&scanner.path) == Answer::Yes)
        {
            continue;
        }
        if ban
            .paths
            .iter()
            .any(|entry| resolved.is_at_or_under(&PathPattern::of(entry.as_str())) == Answer::Yes)
        {
            hits.push(Hit {
                message: Message::from_string(format!("banned by ban set `{}`", ban.id.as_str())),
                rule: banned_name(&ban.id),
            });
        }
    }
}

/// Hits against the config's forbidden references.
fn reference_hits(scanner: &Scanner<'_>, resolved: &Resolved, hits: &mut Vec<Hit>) {
    for forbidden in &scanner.env.config.forbidden_references {
        if !forbidden
            .files
            .iter()
            .any(|glob| glob.matches(&scanner.path) == Answer::Yes)
        {
            continue;
        }
        if forbidden
            .must_not_name
            .iter()
            .any(|entry| resolved.is_at_or_under(&PathPattern::of(entry.as_str())) == Answer::Yes)
        {
            hits.push(hit(
                rules::FORBIDDEN_REFERENCE,
                &Message::of("this file must not name that path"),
            ));
        }
    }
}

/// Collects every rule hit for a resolved path.
fn path_hits(scanner: &Scanner<'_>, resolved: &Resolved) -> Vec<Hit> {
    let mut hits = Vec::new();
    table_hits(scanner, resolved, &mut hits);
    ban_hits(scanner, resolved, &mut hits);
    reference_hits(scanner, resolved, &mut hits);
    hits
}

/// Is a reference to `resolved` allowed by the crate's shell role?
///
/// An `ambient` entry may carry a `files` list of globs (empty means every file
/// of the member), so a shell crate can confine I/O to its I/O module. A path
/// that is merely a prefix of an allowed entry is allowed too, so naming `std::io`
/// on the way to `std::io::stdout` is not a violation by itself. A `pure` member
/// ignores `ambient` entirely.
pub(crate) fn ambient_allowed(scanner: &Scanner<'_>, resolved: &Resolved) -> Answer {
    match scanner.env.member.role {
        Role::Pure => return Answer::No,
        Role::Shell => {}
    }
    scanner
        .env
        .member
        .ambient
        .iter()
        .any(|allow| {
            let files_ok = allow.files.is_empty()
                || allow
                    .files
                    .iter()
                    .any(|glob| glob.matches(&scanner.path) == Answer::Yes);
            let pattern = PathPattern::of(allow.path.as_str());
            files_ok
                && (resolved.is_at_or_under(&pattern) == Answer::Yes
                    || resolved.is_prefix_of(&pattern) == Answer::Yes)
        })
        .answer()
}

/// Flags an ambiguous (possibly glob-imported) name that matches a banned leaf.
fn check_ambiguous(scanner: &mut Scanner<'_>, resolved: &Resolved, span: Span, role: PathRole) {
    let Some(first) = resolved.segments.first() else {
        return;
    };
    if resolved.is_in_table(rules::LEAK_PATHS) == Answer::Yes {
        // A prelude spelling such as `Box::leak` stays flagged when a glob
        // import of another crate might shadow the prelude name.
        let subject =
            Message::from_string(format!("`{}` (via glob import)", resolved.text().as_str()));
        emit_hit(
            scanner,
            span,
            &hit(rules::MEMORY_LEAK, &Message::of(LEAK_MESSAGE)),
            &subject,
        );
    }
    if resolved.segments.len() > 1 && check_through_globs(scanner, resolved, span) == Answer::Yes {
        return;
    }
    let bare_expression = role == PathRole::Expr && resolved.segments.len() == 1;
    if bare_expression || !scanner.env.banned_leaves.contains(first) {
        return;
    }
    for (rule, entry) in candidates_for_leaf(scanner, first) {
        let candidate = synthetic(&Text::of(entry.as_str()));
        if rule.as_str() == rules::AMBIENT_INPUT.name()
            && ambient_allowed(scanner, &candidate) == Answer::Yes
        {
            continue;
        }
        let found = Hit {
            message: Message::of("name may come from a glob import of a non-workspace path"),
            rule,
        };
        let subject = Message::from_string(format!("`{}` (via glob import)", first.as_str()));
        emit_hit(scanner, span, &found, &subject);
    }
}

/// Applies the path rules to a multi-segment path that a glob import may have
/// completed: `use std::*;` then `io::stdin()` is `std::io::stdin`. Answers
/// whether some glob import outside the workspace is in scope; the leaf test of
/// the caller is for paths no such glob explains.
fn check_through_globs(scanner: &mut Scanner<'_>, resolved: &Resolved, span: Span) -> Answer {
    let prefixes = types::external_globs(scanner);
    for prefix in &prefixes {
        let mut segments = prefix.clone();
        segments.extend(resolved.segments.iter().cloned());
        let completed = Resolved {
            kind: Kind::External,
            segments,
        };
        let text = resolved.text();
        let via = Name::from_string(
            prefix
                .iter()
                .map(Name::as_str)
                .collect::<Vec<_>>()
                .join("::"),
        );
        for found in path_hits(scanner, &completed) {
            let subject = Message::from_string(format!(
                "`{}` (via glob import of `{}`)",
                text.as_str(),
                via.as_str()
            ));
            emit_hit(scanner, span, &found, &subject);
        }
    }
    (!prefixes.is_empty()).answer()
}

/// The (rule, banned path) pairs whose last segment is `leaf`.
fn candidates_for_leaf(scanner: &Scanner<'_>, leaf: &Name) -> Vec<(Name, Text)> {
    let tables: [(RuleId, PatternTable); 5] = [
        (rules::UNORDERED_COLLECTION, rules::UNORDERED_PATHS),
        (rules::AMBIENT_INPUT, rules::AMBIENT_PATHS),
        (rules::ABORT_PANIC, rules::PANIC_PATHS),
        (rules::MODULAR_ARITH, rules::MODULAR_TYPE_PATHS),
        (rules::MEMORY_LEAK, rules::LEAK_PATHS),
    ];
    let mut found = Vec::new();
    for (rule, entries) in tables {
        for entry in entries.entries() {
            if entry.rsplit("::").next() == Some(leaf.as_str()) {
                found.push((Name::of(rule.name()), Text::of(entry)));
            }
        }
    }
    for ban in &scanner.env.config.bans {
        if !ban
            .files
            .iter()
            .any(|glob| glob.matches(&scanner.path) == Answer::Yes)
        {
            continue;
        }
        for entry in &ban.paths {
            if entry.as_str().rsplit("::").next() == Some(leaf.as_str()) {
                found.push((banned_name(&ban.id), entry.clone()));
            }
        }
    }
    found
}

/// Method-name rules, for calls and path forms alike.
pub(crate) fn check_method_name(scanner: &mut Scanner<'_>, span: Span, name: &Name) {
    let text = name.as_str();
    if rules::UNWRAP_FAMILY.contains(&text) {
        scanner.report(
            span,
            rules::ABORT_UNWRAP,
            &Message::from_string(format!(
                "`{text}` aborts on failure; propagate the error or match every case"
            )),
        );
    }
    let configured = scanner
        .env
        .config
        .silent_fallback_names
        .iter()
        .any(|extra| extra == name);
    if rules::SILENT_FALLBACK_NAMES.contains(&text) || configured {
        scanner.report(
            span,
            rules::SILENT_FALLBACK,
            &Message::from_string(format!(
                "`{text}` silently substitutes a value; handle both cases explicitly"
            )),
        );
    }
    if text.starts_with("wrapping_")
        || text.starts_with("saturating_")
        || text.starts_with("overflowing_")
    {
        scanner.report(
            span,
            rules::MODULAR_ARITH,
            &Message::from_string(format!("`{text}` hides overflow; use checked operations")),
        );
    }
    if rules::FLOAT_METHODS.contains(&text) {
        scanner.report(
            span,
            rules::FLOAT_TYPE,
            &Message::from_string(format!("`{text}` converts to or from a float")),
        );
    }
    check_banned_method(scanner, span, name);
}

/// Flags a method call named `leak` with no arguments (`Vec::leak`, `String::leak`
/// called as methods). It goes by name because the receiver type is not known.
pub(crate) fn check_leak_method(scanner: &mut Scanner<'_>, call: &syn::ExprMethodCall) {
    if call.method == "leak" && call.args.is_empty() {
        scanner.report(
            call.method.span(),
            rules::MEMORY_LEAK,
            &Message::from_string(format!("`.leak()` call: {LEAK_MESSAGE}")),
        );
    }
}

/// Reports a method call that a ban set in the config names.
fn check_banned_method(scanner: &mut Scanner<'_>, span: Span, name: &Name) {
    let text = name.as_str();
    let ids: Vec<Name> = scanner
        .env
        .config
        .bans
        .iter()
        .filter(|ban| {
            ban.files
                .iter()
                .any(|glob| glob.matches(&scanner.path) == Answer::Yes)
                && ban.methods.contains(name)
        })
        .map(|ban| ban.id.clone())
        .collect();
    for id in ids {
        scanner.report_banned(
            span,
            &id,
            &Message::from_string(format!("method `{text}` is banned by `{}`", id.as_str())),
        );
    }
}

/// Rules that look only at a macro's name.
pub(crate) fn check_macro_name(scanner: &mut Scanner<'_>, path: &syn::Path) {
    let span = first_span(path);
    let resolved = types::resolve_path(scanner, path);
    let Some(last) = path.segments.last() else {
        return;
    };
    let leaf_name = last.ident.to_string();
    let leaf = leaf_name.as_str();
    let builtin = resolved.kind != Kind::Workspace
        && (resolved.segments.len() == 1
            || resolved.segments.first().map(Name::as_str) == Some("std"));
    if builtin {
        check_builtin_macro(scanner, span, &Name::of(leaf));
    }
    let text = resolved.text();
    let ids: Vec<Name> = scanner
        .env
        .config
        .bans
        .iter()
        .filter(|ban| {
            ban.files
                .iter()
                .any(|glob| glob.matches(&scanner.path) == Answer::Yes)
        })
        .filter(|ban| {
            ban.macros.iter().any(|entry| {
                resolved.is_at_or_under(&PathPattern::of(entry.as_str())) == Answer::Yes
                    || (builtin && entry.as_str() == leaf)
            })
        })
        .map(|ban| ban.id.clone())
        .collect();
    for id in ids {
        scanner.report_banned(
            span,
            &id,
            &Message::from_string(format!(
                "macro `{}` is banned by `{}`",
                text.as_str(),
                id.as_str()
            )),
        );
    }
    check_resolved(scanner, &resolved, span, PathRole::Macro);
}

/// Applies the rules specific to built-in macros such as panics, asserts and printing.
fn check_builtin_macro(scanner: &mut Scanner<'_>, span: Span, name: &Name) {
    let leaf = name.as_str();
    if rules::PANIC_MACROS.contains(&leaf) {
        scanner.report(
            span,
            rules::ABORT_PANIC,
            &Message::from_string(format!("`{leaf}!` aborts; return an error")),
        );
    }
    let in_test = match scanner.scope {
        Scope::Test => true,
        Scope::Production | Scope::Example | Scope::Bench | Scope::BuildScript => false,
    };
    if rules::ASSERT_MACROS.contains(&leaf) && !in_test {
        scanner.report(
            span,
            rules::ABORT_ASSERT,
            &Message::from_string(format!("`{leaf}!` aborts outside tests; return an error")),
        );
    }
    let ambient_name = if rules::PRINT_MACROS.contains(&leaf) {
        Some(leaf)
    } else if leaf == "env" || leaf == "option_env" {
        Some("env")
    } else {
        None
    };
    if let Some(ambient) = ambient_name {
        let synth = synthetic(&Text::from_string(format!("std::{ambient}")));
        if ambient_allowed(scanner, &synth) == Answer::No {
            scanner.report(
                span,
                rules::AMBIENT_INPUT,
                &Message::from_string(format!("`{leaf}!` reads or writes ambient state")),
            );
        }
    }
}

/// Builds a resolved external path from `::`-separated text.
pub(crate) fn synthetic(text: &Text) -> Resolved {
    Resolved {
        kind: Kind::External,
        segments: text.as_str().split("::").map(Name::of).collect(),
    }
}

/// Is `expr` visibly a `Path` or `PathBuf`: a constructor call, possibly chained
/// through `join`, `parent` and the like, or parenthesised or borrowed?
fn is_path_value(scanner: &Scanner<'_>, expr: &syn::Expr) -> Answer {
    match expr {
        syn::Expr::Paren(inner) => is_path_value(scanner, &inner.expr),
        syn::Expr::Group(inner) => is_path_value(scanner, &inner.expr),
        syn::Expr::Reference(inner) => is_path_value(scanner, &inner.expr),
        syn::Expr::MethodCall(call) => (PATH_CHAIN.contains(&call.method.to_string().as_str())
            && is_path_value(scanner, &call.receiver) == Answer::Yes)
            .answer(),
        syn::Expr::Call(call) => match call.func.as_ref() {
            syn::Expr::Path(path) => {
                let resolved = types::resolve_path(scanner, &path.path);
                matches!(
                    resolved.text().as_str(),
                    "std::path::Path::new" | "std::path::PathBuf::new" | "std::path::PathBuf::from"
                )
                .answer()
            }
            _ => Answer::No,
        },
        _ => Answer::No,
    }
}

/// Flags a method call that probes the file system through a `Path`.
pub(crate) fn check_probe_method(scanner: &mut Scanner<'_>, call: &syn::ExprMethodCall) {
    let name = call.method.to_string();
    let by_name = PROBE_NAMES.contains(&name.as_str());
    let by_receiver = PROBE_METHODS.contains(&name.as_str())
        && is_path_value(scanner, &call.receiver) == Answer::Yes;
    if !(by_name || by_receiver) {
        return;
    }
    if ambient_allowed(scanner, &synthetic(&Text::of("std::fs"))) == Answer::Yes {
        return;
    }
    scanner.report(
        call.method.span(),
        rules::AMBIENT_INPUT,
        &Message::from_string(format!("`{name}` probes the file system")),
    );
}

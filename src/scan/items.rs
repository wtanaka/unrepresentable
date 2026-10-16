//! Attributes, signatures and item-level rules.
use proc_macro2::Span;

use syn::{Attribute, Expr, ExprLit, Fields, Lit, Meta, ReturnType, Signature, Type, Visibility};

use super::paths;

use super::paths::{PathRole, Qself};

use super::types;

use super::types::{type_name, PrimitiveKind};

use super::{ExpectUse, ImplCtx, Scanner};

use crate::attrs;

use crate::index::{flatten_use, BlockScope, Kind, PathPattern};

use crate::rules;

use crate::types::{Answer, Holds as _, Message, Name, Text};

/// Message used when a `cfg` predicate mentions clippy.
const CFG_CLIPPY_MESSAGE: &str =
    "a cfg predicate mentions clippy; lints must not depend on the tool that runs them";

/// Whether a method may touch a newtype's primitive.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Exemption {
    /// The method may touch the primitive of a newtype.
    Exempt,
    /// The signature must not expose a bare primitive.
    Required,
}

/// Where in a signature a type appears.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    /// The type appears in a parameter.
    Parameter,
    /// The type appears in the return type.
    Return,
}

/// Where a fn's signature rules report, and whether it may touch a newtype's primitive.
struct FnSite<'site> {
    /// Whether the fn may touch a newtype's primitive.
    exemption: Exemption,
    /// The fn's name.
    name: &'site Name,
    /// Span at which signature findings are reported.
    span: Span,
}

/// One path leaf of a `use` tree and where it sits.
struct UseLeaf {
    /// The resolved path segments of the leaf.
    segments: Vec<Name>,
    /// Span of the leaf in the source.
    span: Span,
}

/// Does the signature type mention a primitive that must be wrapped? A bool
/// parameter has its own rule (`bool-param`); a bool in a return position
/// belongs here.
fn mentions_primitive(kinds: &[PrimitiveKind], position: Position) -> Answer {
    kinds
        .iter()
        .any(|kind| match kind {
            PrimitiveKind::Integer
            | PrimitiveKind::Float
            | PrimitiveKind::Text
            | PrimitiveKind::Char => true,
            PrimitiveKind::Bool => position == Position::Return,
        })
        .answer()
}

/// Applies attribute rules to one attribute, expanding `cfg_attr` first.
pub(crate) fn on_attribute(scanner: &mut Scanner<'_>, attr: &Attribute) {
    let span = attr.pound_token.span;
    attrs::expand(attr, &mut |meta, preds| {
        on_expanded_meta(scanner, meta, preds, span);
    });
}

/// Applies attribute rules to one attribute after `cfg_attr` expansion.
pub(crate) fn on_expanded_meta(
    scanner: &mut Scanner<'_>,
    meta: &Meta,
    preds: &[&Meta],
    span: Span,
) {
    if preds
        .iter()
        .any(|pred| attrs::pred_mentions_clippy(pred) == Answer::Yes)
    {
        scanner.report(span, rules::CFG_CLIPPY, &Message::of(CFG_CLIPPY_MESSAGE));
    }
    let name = attrs::meta_name(meta);
    let shown = name.as_str();
    match shown {
        "cfg" => {
            if attrs::pred_mentions_clippy(meta) == Answer::Yes {
                scanner.report(span, rules::CFG_CLIPPY, &Message::of(CFG_CLIPPY_MESSAGE));
            }
        }
        "allow" | "warn" => scanner.report(
            span,
            rules::SUPPRESSION,
            &Message::from_string(format!(
                "`#[{shown}(..)]` is not allowed; fix the code or record a ledger-backed `#[expect]`"
            )),
        ),
        "expect" => on_expect(scanner, meta, span),
        "ignore" | "should_panic" => scanner.report(
            span,
            rules::TEST_ATTRIBUTES,
            &Message::from_string(format!(
                "`#[{shown}]` is not allowed; move long or manual runs to an example binary"
            )),
        ),
        "derive" => on_derive(scanner, meta),
        _ => {}
    }
    match meta {
        Meta::Path(path) | Meta::List(syn::MetaList { path, .. }) => {
            paths::check_path(scanner, path, Qself::Absent, PathRole::Attr);
        }
        Meta::NameValue(name_value) => {
            paths::check_path(scanner, &name_value.path, Qself::Absent, PathRole::Attr);
        }
    }
}

/// Flags `Default` in a `#[derive(..)]` (also inside `cfg_attr`) on a struct or union.
pub(crate) fn on_default_derive(scanner: &mut Scanner<'_>, attrs_list: &[Attribute]) {
    let mut spans: Vec<Span> = Vec::new();
    for attr in attrs_list {
        attrs::expand(attr, &mut |meta, _preds| {
            if attrs::meta_name(meta).as_str() != "derive" {
                return;
            }
            let Some(listed) = attrs::nested(meta) else {
                return;
            };
            for item in listed {
                let Meta::Path(path) = &item else {
                    continue;
                };
                let resolved = types::resolve_path(scanner, path);
                let std_default = match resolved.kind {
                    Kind::Ambiguous | Kind::External | Kind::Unresolved => {
                        matches!(
                            resolved.text().as_str(),
                            "Default" | "std::default::Default"
                        )
                    }
                    Kind::Local | Kind::Variant | Kind::Workspace => false,
                };
                if let (true, Some(last)) = (std_default, path.segments.last()) {
                    spans.push(last.ident.span());
                }
            }
        });
    }
    for span in spans {
        scanner.report(
            span,
            rules::DEFAULT_CONSTRUCTION,
            &Message::of(
                "derived `Default` fills every field with a blank value; build the value with a named constructor, or write `impl Default` by hand with deliberate values",
            ),
        );
    }
}

/// Checks the traits named in a `#[derive(..)]` against the config's ban sets.
fn on_derive(scanner: &mut Scanner<'_>, meta: &Meta) {
    let Some(items) = attrs::nested(meta) else {
        return;
    };
    for item in &items {
        let Meta::Path(path) = item else {
            continue;
        };
        paths::check_path(scanner, path, Qself::Absent, PathRole::Attr);
        let Some(last) = path.segments.last() else {
            continue;
        };
        let derive = Name::from_string(last.ident.to_string());
        let ids: Vec<Name> = scanner
            .env
            .config
            .bans
            .iter()
            .filter(|ban| {
                ban.files
                    .iter()
                    .any(|glob| glob.matches(&scanner.path) == Answer::Yes)
                    && ban.derives.contains(&derive)
            })
            .map(|ban| ban.id.clone())
            .collect();
        for id in ids {
            scanner.report_banned(
                last.ident.span(),
                &id,
                &Message::from_string(format!(
                    "derive `{}` is banned by `{}`",
                    derive.as_str(),
                    id.as_str()
                )),
            );
        }
    }
}

/// Checks an `#[expect(..)]` attribute for a reason and records its lints for the ledger.
fn on_expect(scanner: &mut Scanner<'_>, meta: &Meta, span: Span) {
    let Some(items) = attrs::nested(meta) else {
        scanner.report(
            span,
            rules::SUPPRESSION,
            &Message::of("`#[expect(..)]` arguments could not be parsed"),
        );
        return;
    };
    let mut lints: Vec<Text> = Vec::new();
    let mut reason_ok = false;
    for item in &items {
        match item {
            Meta::Path(path) => lints.push(attrs::path_text(path)),
            Meta::NameValue(name_value) if name_value.path.is_ident("reason") => {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(text),
                    ..
                }) = &name_value.value
                {
                    reason_ok = !text.value().trim().is_empty();
                }
            }
            Meta::NameValue(_) | Meta::List(_) => {}
        }
    }
    if !reason_ok {
        scanner.report(
            span,
            rules::SUPPRESSION,
            &Message::of("`#[expect(..)]` needs `reason` as a non-empty string literal"),
        );
    }
    let (line, col) = super::span_line_col(span);
    let item = scanner.item_text();
    for lint in lints {
        scanner.expects.push(ExpectUse {
            col,
            item: item.clone(),
            line,
            lint,
            path: scanner.path.clone(),
        });
    }
}

/// Does `attr` mark a test fn, by resolved path?
pub(crate) fn is_test_attr(scanner: &Scanner<'_>, attr: &Attribute) -> Answer {
    is_test_path(scanner, attr.path())
}

/// Does `path` name a test attribute, either as written or once resolved?
pub(crate) fn is_test_path(scanner: &Scanner<'_>, path: &syn::Path) -> Answer {
    let raw = attrs::path_text(path);
    let resolved = types::resolve_path(scanner, path).text();
    let configured = scanner
        .env
        .config
        .test_attributes
        .iter()
        .map(|text| text.as_str().to_owned());
    rules::BUILTIN_TEST_ATTRIBUTES
        .iter()
        .map(|text| (*text).to_owned())
        .chain(configured)
        .any(|entry| entry == raw.as_str() || entry == resolved.as_str())
        .answer()
}

/// Does any attribute in `attrs_list` mark the fn as a test?
pub(crate) fn is_test_fn(scanner: &Scanner<'_>, attrs_list: &[Attribute]) -> Answer {
    attrs_list
        .iter()
        .any(|attr| is_test_attr(scanner, attr) == Answer::Yes)
        .answer()
}

/// Signature rules for any fn, method or trait method.
pub(crate) fn on_signature(
    scanner: &mut Scanner<'_>,
    sig: &Signature,
    attrs_list: &[Attribute],
    vis: &Visibility,
) {
    let span = sig.ident.span();
    let name = Name::from_string(sig.ident.to_string());
    check_fn_name(scanner, span, &name);
    let typed: Vec<&syn::PatType> = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(typed) => Some(typed),
            syn::FnArg::Receiver(_) => None,
        })
        .collect();
    let exemption = signature_exempt(scanner, &name, vis);
    for param in &typed {
        let mut kinds = Vec::new();
        types::collect_primitives(scanner, &param.ty, &mut kinds);
        if kinds.contains(&PrimitiveKind::Bool) {
            scanner.report(
                span,
                rules::BOOL_PARAM,
                &Message::from_string(format!(
                    "fn `{}` takes a bool parameter; use a two-variant enum",
                    name.as_str()
                )),
            );
        }
        if exemption == Exemption::Required
            && mentions_primitive(&kinds, Position::Parameter) == Answer::Yes
        {
            scanner.report(
                span,
                rules::PRIMITIVE_SIGNATURE,
                &Message::from_string(format!(
                    "fn `{}` takes a bare primitive; wrap it in a newtype",
                    name.as_str()
                )),
            );
        }
    }
    for pair in typed.windows(2) {
        if let [left, right] = pair {
            if types::type_key(scanner, &left.ty) == types::type_key(scanner, &right.ty) {
                scanner.report(
                    span,
                    rules::TRANSPOSABLE_PARAMS,
                    &Message::from_string(format!(
                        "fn `{}` has adjacent parameters of the same type; callers can swap them",
                        name.as_str()
                    )),
                );
            }
        }
    }
    if let ReturnType::Type(_, ty) = &sig.output {
        let site = FnSite {
            exemption,
            name: &name,
            span,
        };
        check_return_type(scanner, &site, ty, attrs_list);
    }
}

/// Flags fn names that advertise an unchecked or raw escape hatch.
fn check_fn_name(scanner: &mut Scanner<'_>, span: Span, name: &Name) {
    let configured = scanner
        .env
        .config
        .fn_name_suffixes
        .iter()
        .any(|suffix| name.as_str().ends_with(suffix.as_str()));
    if name.as_str().ends_with("_raw") || name.as_str().ends_with("_unchecked") || configured {
        scanner.report(
            span,
            rules::PUB_FN_NAME,
            &Message::from_string(format!(
                "fn name `{}` advertises an unchecked or raw escape hatch",
                name.as_str()
            )),
        );
    }
}

/// Applies return-type rules: bare primitives and `Option` without `#[must_use]`.
fn check_return_type(
    scanner: &mut Scanner<'_>,
    site: &FnSite<'_>,
    ty: &Type,
    attrs_list: &[Attribute],
) {
    let (span, name) = (site.span, site.name);
    let mut kinds = Vec::new();
    types::collect_primitives(scanner, ty, &mut kinds);
    if site.exemption == Exemption::Required
        && mentions_primitive(&kinds, Position::Return) == Answer::Yes
    {
        scanner.report(
            span,
            rules::PRIMITIVE_SIGNATURE,
            &Message::from_string(format!(
                "fn `{}` returns a bare primitive; wrap it in a newtype",
                name.as_str()
            )),
        );
    }
    let must_use = attrs_list
        .iter()
        .any(|attr| attr.path().is_ident("must_use"));
    if is_option_type(scanner, ty) == Answer::Yes && !must_use {
        scanner.report(
            span,
            rules::OPTION_NOT_MUST_USE,
            &Message::from_string(format!(
                "fn `{}` returns Option without #[must_use]",
                name.as_str()
            )),
        );
    }
}

/// Is `ty` the `Option` type, looking through parentheses and groups?
fn is_option_type(scanner: &Scanner<'_>, ty: &Type) -> Answer {
    match ty {
        Type::Path(type_path) => {
            let resolved = types::resolve_path(scanner, &type_path.path);
            let names: Vec<&str> = resolved.segments.iter().map(Name::as_str).collect();
            matches!(names.as_slice(), ["Option"] | ["std", "option", "Option"]).answer()
        }
        Type::Paren(paren) => is_option_type(scanner, &paren.elem),
        Type::Group(group) => is_option_type(scanner, &group.elem),
        _ => Answer::No,
    }
}

/// Constructors and accessors of a newtype may touch the primitive.
fn signature_exempt(scanner: &Scanner<'_>, name: &Name, vis: &Visibility) -> Exemption {
    let owner = match &scanner.impl_ctx {
        ImplCtx::Inherent(found) => found.as_ref(),
        ImplCtx::Free | ImplCtx::TraitImpl => None,
    };
    let public = matches!(vis, Visibility::Public(_));
    let public_listing = format!("pub {}", name.as_str());
    let listed = owner.is_some_and(|type_name| {
        scanner
            .env
            .member
            .newtypes
            .iter()
            .filter(|entry| &entry.type_name == type_name)
            .any(|entry| {
                entry
                    .methods
                    .iter()
                    .any(|item| (item == name && !public) || item.as_str() == public_listing)
            })
    });
    if listed {
        Exemption::Exempt
    } else {
        Exemption::Required
    }
}

/// Applies field rules to the fields of a struct or variant if they are named.
pub(crate) fn on_named_fields(scanner: &mut Scanner<'_>, fields: &Fields) {
    if let Fields::Named(named) = fields {
        on_field_list(scanner, named.named.iter());
    }
}

/// Flags fields whose type holds a bare primitive.
pub(crate) fn on_field_list<'fld>(
    scanner: &mut Scanner<'_>,
    fields: impl Iterator<Item = &'fld syn::Field>,
) {
    for field in fields {
        let mut kinds = Vec::new();
        types::collect_primitives(scanner, &field.ty, &mut kinds);
        if let (false, Some(ident)) = (kinds.is_empty(), &field.ident) {
            scanner.push_item(Text::from_string(format!("field {ident}")));
            scanner.report(
                ident.span(),
                rules::PRIMITIVE_FIELD,
                &Message::from_string(format!(
                    "field `{ident}` holds a bare primitive; wrap it in a newtype"
                )),
            );
            scanner.pop_item();
        }
    }
}

/// Builds a label such as `impl Trait for Type` describing an `impl` block.
pub(crate) fn impl_label(item: &syn::ItemImpl) -> Text {
    let self_name = type_name(&item.self_ty);
    match &item.trait_ {
        Some((_, path, _)) => Text::from_string(format!(
            "impl {} for {}",
            types::path_name(path).as_str(),
            self_name.as_str()
        )),
        None => Text::from_string(format!("impl {}", self_name.as_str())),
    }
}

/// Checks the trait of an `impl` block against the config's ban sets.
pub(crate) fn on_impl_traits(scanner: &mut Scanner<'_>, item: &syn::ItemImpl) {
    let Some((_, path, _)) = &item.trait_ else {
        return;
    };
    paths::check_path(scanner, path, Qself::Absent, PathRole::Type);
    let resolved = types::resolve_path(scanner, path);
    let ids: Vec<Name> = scanner
        .env
        .config
        .bans
        .iter()
        .filter(|ban| {
            ban.files
                .iter()
                .any(|glob| glob.matches(&scanner.path) == Answer::Yes)
                && ban.impl_traits.iter().any(|entry| {
                    resolved.is_at_or_under(&PathPattern::of(entry.as_str())) == Answer::Yes
                })
        })
        .map(|ban| ban.id.clone())
        .collect();
    for id in ids {
        scanner.report_banned(
            paths::first_span(path),
            &id,
            &Message::from_string(format!(
                "impl of `{}` is banned by `{}`",
                resolved.text().as_str(),
                id.as_str()
            )),
        );
    }
}

/// Classifies an `impl` block as a trait impl, a newtype's inherent impl or other.
pub(crate) fn impl_ctx_for(scanner: &Scanner<'_>, item: &syn::ItemImpl) -> ImplCtx {
    if item.trait_.is_some() {
        return ImplCtx::TraitImpl;
    }
    if let Type::Path(type_path) = item.self_ty.as_ref() {
        if type_path.qself.is_none() && type_path.path.segments.len() == 1 {
            if let Some(segment) = type_path.path.segments.first() {
                let name = Name::from_string(segment.ident.to_string());
                if scanner.env.newtypes.contains(&name) {
                    return ImplCtx::Inherent(Some(name));
                }
            }
        }
    }
    ImplCtx::Inherent(None)
}

/// Resolves the self type of an `impl` block to path segments, if it is a plain path.
#[must_use]
pub(crate) fn self_path_of(scanner: &Scanner<'_>, item: &syn::ItemImpl) -> Option<Vec<Name>> {
    match item.self_ty.as_ref() {
        Type::Path(type_path) if type_path.qself.is_none() => {
            Some(types::resolve_path(scanner, &type_path.path).segments)
        }
        _ => None,
    }
}

/// Checks every leaf of a `use` tree against the path rules.
pub(crate) fn on_use_tree(scanner: &mut Scanner<'_>, tree: &syn::UseTree) {
    let mut leaves: Vec<UseLeaf> = Vec::new();
    collect_use_leaves(tree, &mut Vec::new(), &mut leaves);
    for leaf in leaves {
        let resolved = types::resolve_segments(scanner, &leaf.segments);
        paths::check_resolved(scanner, &resolved, leaf.span, PathRole::Use);
    }
}

/// Scans a block's `use` statements and local items into a block scope.
pub(crate) fn block_scope(stmts: &[syn::Stmt]) -> BlockScope {
    let mut scope = BlockScope::empty();
    for stmt in stmts {
        let syn::Stmt::Item(item) = stmt else {
            continue;
        };
        let declared = match item {
            syn::Item::Use(item_use) => {
                let mut bindings = Vec::new();
                let mut globs = Vec::new();
                flatten_use(&item_use.tree, &mut Vec::new(), &mut bindings, &mut globs);
                scope.uses.extend(bindings);
                scope.globs.extend(globs);
                None
            }
            syn::Item::Struct(inner) => Some(inner.ident.to_string()),
            syn::Item::Enum(inner) => {
                let variants = inner
                    .variants
                    .iter()
                    .map(|variant| Name::from_string(variant.ident.to_string()))
                    .collect();
                scope
                    .enums
                    .extend([(Name::from_string(inner.ident.to_string()), variants)]);
                Some(inner.ident.to_string())
            }
            syn::Item::Fn(inner) => Some(inner.sig.ident.to_string()),
            syn::Item::Const(inner) => Some(inner.ident.to_string()),
            syn::Item::Type(inner) => Some(inner.ident.to_string()),
            _ => None,
        };
        if let Some(name) = declared {
            scope.items.extend([Name::from_string(name)]);
        }
    }
    scope
}

/// Flattens a `use` tree into its leaves, each with its full path prefix.
fn collect_use_leaves(tree: &syn::UseTree, prefix: &mut Vec<Name>, out: &mut Vec<UseLeaf>) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(Name::from_string(path.ident.to_string()));
            collect_use_leaves(&path.tree, prefix, out);
            match prefix.pop() {
                Some(_) | None => {}
            }
        }
        syn::UseTree::Name(name) => {
            let mut full = prefix.clone();
            if name.ident != "self" {
                full.push(Name::from_string(name.ident.to_string()));
            }
            out.push(UseLeaf {
                segments: full,
                span: name.ident.span(),
            });
        }
        syn::UseTree::Rename(rename) => {
            let mut full = prefix.clone();
            if rename.ident != "self" {
                full.push(Name::from_string(rename.ident.to_string()));
            }
            out.push(UseLeaf {
                segments: full,
                span: rename.ident.span(),
            });
        }
        syn::UseTree::Glob(glob) => out.push(UseLeaf {
            segments: prefix.clone(),
            span: glob.star_token.spans[0],
        }),
        syn::UseTree::Group(group) => {
            for inner in &group.items {
                collect_use_leaves(inner, prefix, out);
            }
        }
    }
}

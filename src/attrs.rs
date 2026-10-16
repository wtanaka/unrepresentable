//! Attribute and `cfg` predicate helpers.
use syn::parse::Parser;

use syn::punctuated::Punctuated;

use syn::{Attribute, Meta, Token};

use crate::types::{Answer, Holds as _, Text};

/// The receiver of the attributes `expand` finds.
type Visit<'visit> = dyn FnMut(&Meta, &[&Meta]) + 'visit;

/// The nested metas of `name(..)`, or empty when the tokens do not parse.
#[must_use]
pub(crate) fn nested(meta: &Meta) -> Option<Vec<Meta>> {
    let Meta::List(list) = meta else {
        return None;
    };
    let parsed: Result<Punctuated<Meta, Token![,]>, _> =
        Parser::parse2(Punctuated::parse_terminated, list.tokens.clone());
    match parsed {
        Ok(punct) => Some(punct.into_iter().collect()),
        Err(_) => None,
    }
}

/// The `::`-joined text of a path.
pub(crate) fn path_text(path: &syn::Path) -> Text {
    Text::from_string(
        path.segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
    )
}

/// `test` or `all(test, ..)` (at any depth of `all`).
pub(crate) fn pred_is_test(meta: &Meta) -> Answer {
    match meta {
        Meta::Path(path) => path.is_ident("test").answer(),
        Meta::List(list) if list.path.is_ident("all") => match nested(meta) {
            Some(items) => items
                .iter()
                .any(|item| pred_is_test(item) == Answer::Yes)
                .answer(),
            None => Answer::No,
        },
        Meta::List(_) | Meta::NameValue(_) => Answer::No,
    }
}

/// Does the predicate mention `clippy` anywhere?
pub(crate) fn pred_mentions_clippy(meta: &Meta) -> Answer {
    match meta {
        Meta::Path(path) => path.is_ident("clippy").answer(),
        Meta::NameValue(name_value) => (name_value.path.is_ident("clippy") || {
            matches!(&name_value.value, syn::Expr::Lit(lit)
                    if matches!(&lit.lit, syn::Lit::Str(text) if text.value().contains("clippy")))
        })
        .answer(),
        Meta::List(list) => (list.path.is_ident("clippy")
            || match nested(meta) {
                Some(items) => items
                    .iter()
                    .any(|item| pred_mentions_clippy(item) == Answer::Yes),
                None => list.tokens.to_string().contains("clippy"),
            })
        .answer(),
    }
}

/// Calls `visit` for each attribute after `cfg_attr` expansion, at any
/// nesting, with the `cfg_attr` predicates that guard it.
pub(crate) fn expand(attr: &Attribute, visit: &mut Visit<'_>) {
    expand_meta(&attr.meta, &[], visit);
}

/// The predicate and attributes inside `cfg_attr(pred, attrs..)`.
#[must_use]
fn cfg_attr_items(meta: &Meta) -> Option<Vec<Meta>> {
    let Meta::List(list) = meta else {
        return None;
    };
    if list.path.is_ident("cfg_attr") {
        nested(meta)
    } else {
        None
    }
}

/// Recursive step of `expand`: unwraps `cfg_attr` and calls `visit` on the leaf metas.
pub(crate) fn expand_meta(meta: &Meta, preds: &[&Meta], visit: &mut Visit<'_>) {
    if let Some(items) = cfg_attr_items(meta) {
        if let Some((pred, rest)) = items.split_first() {
            let mut deeper: Vec<&Meta> = preds.to_vec();
            deeper.push(pred);
            for inner in rest {
                expand_meta(inner, &deeper, visit);
            }
            return;
        }
    }
    visit(meta, preds);
}

/// The path text naming an attribute meta.
pub(crate) fn meta_name(meta: &Meta) -> Text {
    path_text(meta.path())
}

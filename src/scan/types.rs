//! Type utilities: primitive detection, canonical keys, display names.

use syn::{GenericArgument, PathArguments, ReturnType, Type, TypeParamBound};

use super::Scanner;
use crate::index::{raw_segments, Kind, Query, Resolved};
use crate::rules;
use crate::types::{Name, Text};

/// The kind of bare primitive a type names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PrimitiveKind {
    /// `bool`.
    Bool,
    /// `char`.
    Char,
    /// `f32` or `f64`.
    Float,
    /// One of the integer primitives.
    Integer,
    /// `String` or `str`.
    Text,
}

/// Resolves already-split path segments in the scanner's current scope.
pub(crate) fn resolve_segments(scanner: &Scanner<'_>, segments: &[Name]) -> Resolved {
    scanner.env.resolver.resolve(&Query {
        blocks: &scanner.blocks,
        module: &scanner.module,
        segments,
        self_ty: scanner.self_ty.as_deref(),
    })
}

/// The paths of the non-workspace crates and modules whose names a glob import
/// in scope brings in (`use std::*;` gives `std`).
pub(crate) fn external_globs(scanner: &Scanner<'_>) -> Vec<Vec<Name>> {
    scanner.env.resolver.external_globs(&Query {
        blocks: &scanner.blocks,
        module: &scanner.module,
        segments: &[],
        self_ty: scanner.self_ty.as_deref(),
    })
}

/// Resolves a `syn` path in the scanner's current scope.
pub(crate) fn resolve_path(scanner: &Scanner<'_>, path: &syn::Path) -> Resolved {
    let segments = raw_segments(path);
    let mut resolved = resolve_segments(scanner, &segments);
    if path.leading_colon.is_some() && resolved.segments.len() == segments.len() {
        resolved.kind = Kind::External;
    }
    resolved
}

/// The primitive `path` names, if it names one bare.
#[must_use]
pub(crate) fn primitive_of(scanner: &Scanner<'_>, path: &syn::Path) -> Option<PrimitiveKind> {
    let resolved = resolve_path(scanner, path);
    let names: Vec<&str> = resolved.segments.iter().map(Name::as_str).collect();
    let leaf = match names.as_slice() {
        [only] => *only,
        ["std", "string", "String"] => "String",
        ["std", "primitive", leaf] => *leaf,
        _ => return None,
    };
    if rules::INTEGER_PRIMITIVES.contains(&leaf) {
        Some(PrimitiveKind::Integer)
    } else if leaf == "f32" || leaf == "f64" {
        Some(PrimitiveKind::Float)
    } else if leaf == "bool" {
        Some(PrimitiveKind::Bool)
    } else if leaf == "char" {
        Some(PrimitiveKind::Char)
    } else if leaf == "String" || leaf == "str" {
        Some(PrimitiveKind::Text)
    } else {
        None
    }
}

/// Collects bare primitives at the top level or nested in generics,
/// references, slices, arrays, tuples, pointers and fn types.
pub(crate) fn collect_primitives(scanner: &Scanner<'_>, ty: &Type, out: &mut Vec<PrimitiveKind>) {
    match ty {
        Type::Path(type_path) => {
            if type_path.qself.is_none() {
                if let Some(kind) = primitive_of(scanner, &type_path.path) {
                    out.push(kind);
                    return;
                }
            }
            if let Some(qself) = &type_path.qself {
                collect_primitives(scanner, &qself.ty, out);
            }
            for segment in &type_path.path.segments {
                collect_in_arguments(scanner, &segment.arguments, out);
            }
        }
        Type::Reference(reference) => collect_primitives(scanner, &reference.elem, out),
        Type::Slice(slice) => collect_primitives(scanner, &slice.elem, out),
        Type::Array(array) => collect_primitives(scanner, &array.elem, out),
        Type::Ptr(pointer) => collect_primitives(scanner, &pointer.elem, out),
        Type::Paren(paren) => collect_primitives(scanner, &paren.elem, out),
        Type::Group(group) => collect_primitives(scanner, &group.elem, out),
        Type::Tuple(tuple) => {
            for element in &tuple.elems {
                collect_primitives(scanner, element, out);
            }
        }
        Type::BareFn(bare) => {
            for input in &bare.inputs {
                collect_primitives(scanner, &input.ty, out);
            }
            if let ReturnType::Type(_, output) = &bare.output {
                collect_primitives(scanner, output, out);
            }
        }
        Type::ImplTrait(imp) => collect_in_bounds(scanner, imp.bounds.iter(), out),
        Type::TraitObject(object) => collect_in_bounds(scanner, object.bounds.iter(), out),
        _ => {}
    }
}

/// Collects primitives from the generic arguments of trait bounds.
fn collect_in_bounds<'bound>(
    scanner: &Scanner<'_>,
    bounds: impl Iterator<Item = &'bound TypeParamBound>,
    out: &mut Vec<PrimitiveKind>,
) {
    for bound in bounds {
        if let TypeParamBound::Trait(trait_bound) = bound {
            for segment in &trait_bound.path.segments {
                collect_in_arguments(scanner, &segment.arguments, out);
            }
        }
    }
}

/// Collects primitives from the generic arguments of one path segment.
fn collect_in_arguments(
    scanner: &Scanner<'_>,
    arguments: &PathArguments,
    out: &mut Vec<PrimitiveKind>,
) {
    match arguments {
        PathArguments::None => {}
        PathArguments::AngleBracketed(angle) => {
            for argument in &angle.args {
                match argument {
                    GenericArgument::Type(inner) => collect_primitives(scanner, inner, out),
                    GenericArgument::AssocType(assoc) => {
                        collect_primitives(scanner, &assoc.ty, out);
                    }
                    _ => {}
                }
            }
        }
        PathArguments::Parenthesized(paren) => {
            for input in &paren.inputs {
                collect_primitives(scanner, input, out);
            }
            if let ReturnType::Type(_, output) = &paren.output {
                collect_primitives(scanner, output, out);
            }
        }
    }
}

/// Canonical key: equal keys mean the same type after alias resolution.
pub(crate) fn type_key(scanner: &Scanner<'_>, ty: &Type) -> Text {
    let key = match ty {
        Type::Path(type_path) => {
            let resolved = resolve_path(scanner, &type_path.path);
            let mut key = resolved.text().as_str().to_owned();
            if let Some(last) = type_path.path.segments.last() {
                key.push_str(arguments_key(scanner, &last.arguments).as_str());
            }
            if type_path.qself.is_some() {
                key.push_str("<qself>");
            }
            key
        }
        Type::Reference(reference) => {
            let marker = if reference.mutability.is_some() {
                "&mut "
            } else {
                "&"
            };
            format!("{marker}{}", type_key(scanner, &reference.elem).as_str())
        }
        Type::Slice(slice) => format!("[{}]", type_key(scanner, &slice.elem).as_str()),
        Type::Array(array) => format!("[{}; _]", type_key(scanner, &array.elem).as_str()),
        Type::Ptr(pointer) => format!("*{}", type_key(scanner, &pointer.elem).as_str()),
        Type::Paren(paren) => type_key(scanner, &paren.elem).as_str().to_owned(),
        Type::Group(group) => type_key(scanner, &group.elem).as_str().to_owned(),
        Type::Tuple(tuple) => format!(
            "({})",
            tuple
                .elems
                .iter()
                .map(|element| type_key(scanner, element).as_str().to_owned())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::ImplTrait(imp) => format!("impl {}", bounds_key(scanner, imp.bounds.iter()).as_str()),
        Type::TraitObject(object) => {
            format!("dyn {}", bounds_key(scanner, object.bounds.iter()).as_str())
        }
        Type::BareFn(_) => "fn".to_owned(),
        Type::Never(_) => "!".to_owned(),
        Type::Infer(_) => "_".to_owned(),
        _ => "?".to_owned(),
    };
    Text::from_string(key)
}

/// Builds the canonical key of a list of bounds, joining trait keys with ` + `.
fn bounds_key<'bound>(
    scanner: &Scanner<'_>,
    bounds: impl Iterator<Item = &'bound TypeParamBound>,
) -> Text {
    let keys: Vec<String> = bounds
        .filter_map(|bound| match bound {
            TypeParamBound::Trait(trait_bound) => {
                let resolved = resolve_path(scanner, &trait_bound.path);
                let mut key = resolved.text().as_str().to_owned();
                if let Some(last) = trait_bound.path.segments.last() {
                    key.push_str(arguments_key(scanner, &last.arguments).as_str());
                }
                Some(key)
            }
            _ => None,
        })
        .collect();
    Text::from_string(keys.join(" + "))
}

/// Builds the canonical key of a path segment's generic arguments.
fn arguments_key(scanner: &Scanner<'_>, arguments: &PathArguments) -> Text {
    let key = match arguments {
        PathArguments::None => String::new(),
        PathArguments::AngleBracketed(angle) => {
            let parts: Vec<String> = angle
                .args
                .iter()
                .filter_map(|argument| match argument {
                    GenericArgument::Type(inner) => {
                        Some(type_key(scanner, inner).as_str().to_owned())
                    }
                    _ => None,
                })
                .collect();
            format!("<{}>", parts.join(", "))
        }
        PathArguments::Parenthesized(paren) => {
            let inputs: Vec<String> = paren
                .inputs
                .iter()
                .map(|input| type_key(scanner, input).as_str().to_owned())
                .collect();
            let output = match &paren.output {
                ReturnType::Default => String::new(),
                ReturnType::Type(_, ty) => format!(" -> {}", type_key(scanner, ty).as_str()),
            };
            format!("({}){output}", inputs.join(", "))
        }
    };
    Text::from_string(key)
}
/// A readable name for a path in item labels: identifiers and their type
/// arguments, never source text, so `From<A>` and `From<B>` differ.
pub(crate) fn path_name(path: &syn::Path) -> Text {
    Text::from_string(
        path.segments
            .iter()
            .map(|segment| {
                let mut part = segment.ident.to_string();
                if let PathArguments::AngleBracketed(angle) = &segment.arguments {
                    let inner: Vec<String> = angle
                        .args
                        .iter()
                        .filter_map(|argument| match argument {
                            GenericArgument::Type(inner) => {
                                Some(type_name(inner).as_str().to_owned())
                            }
                            _ => None,
                        })
                        .collect();
                    if !inner.is_empty() {
                        part.extend([format!("<{}>", inner.join(", "))]);
                    }
                }
                part
            })
            .collect::<Vec<_>>()
            .join("::"),
    )
}

/// A readable name for item labels: identifiers only, never source text.
pub(crate) fn type_name(ty: &Type) -> Text {
    let text = match ty {
        Type::Path(type_path) => {
            let mut text = path_name(&type_path.path).as_str().to_owned();
            if type_path.qself.is_some() {
                text = format!("<_>::{text}");
            }
            text
        }
        Type::Reference(reference) => {
            let marker = if reference.mutability.is_some() {
                "&mut "
            } else {
                "&"
            };
            format!("{marker}{}", type_name(&reference.elem).as_str())
        }
        Type::Slice(slice) => format!("[{}]", type_name(&slice.elem).as_str()),
        Type::Array(array) => format!("[{}; _]", type_name(&array.elem).as_str()),
        Type::Tuple(tuple) => format!(
            "({})",
            tuple
                .elems
                .iter()
                .map(|element| type_name(element).as_str().to_owned())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::Paren(paren) => type_name(&paren.elem).as_str().to_owned(),
        Type::Group(group) => type_name(&group.elem).as_str().to_owned(),
        Type::Ptr(pointer) => format!("*{}", type_name(&pointer.elem).as_str()),
        _ => "_".to_owned(),
    };
    Text::from_string(text)
}

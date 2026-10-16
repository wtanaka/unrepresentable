//! Expression-level rules.

use proc_macro2::Span;
use syn::token::Else;
use syn::visit::{self, Visit};
use syn::{BinOp, Expr, Lit, Pat, Type, UnOp};

use super::paths;
use super::paths::qself_of;
use super::paths::PathRole;
use super::patterns;
use super::types;
use super::types::PrimitiveKind;
use super::{AssertionSeen, Body, Scanner};
use crate::findings::Finding;
use crate::index::Kind;
use crate::rules;
use crate::types::{Answer, Holds as _, Message, Name};

/// Message of every `test-early-return` finding.
const EARLY_RETURN_MESSAGE: &str = "`return` before any assertion lets the test pass without asserting; assert before returning, or fail the test instead of skipping it";

/// Finds any occurrence of one name, as an identifier or inside macro tokens.
struct Occurrence<'name> {
    /// The name looked for.
    name: &'name Name,
    /// Whether it has been seen.
    seen: Answer,
}

impl<'ast> Visit<'ast> for Occurrence<'_> {
    fn visit_ident(&mut self, ident: &'ast syn::Ident) {
        if ident == self.name.as_str() {
            self.seen = Answer::Yes;
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if tokens_mention(&mac.tokens, self.name) == Answer::Yes {
            self.seen = Answer::Yes;
        }
        visit::visit_macro(self, mac);
    }
}

/// Returns the span of the operator token of an arithmetic or bit binary operator, if any.
#[must_use]
const fn binop_span(op: &BinOp) -> Option<Span> {
    match op {
        BinOp::Add(token) => Some(token.spans[0]),
        BinOp::Sub(token) => Some(token.spans[0]),
        BinOp::Mul(token) => Some(token.spans[0]),
        BinOp::Div(token) => Some(token.spans[0]),
        BinOp::Rem(token) => Some(token.spans[0]),
        BinOp::BitXor(token) => Some(token.spans[0]),
        BinOp::BitAnd(token) => Some(token.spans[0]),
        BinOp::BitOr(token) => Some(token.spans[0]),
        BinOp::Shl(token) => Some(token.spans[0]),
        BinOp::Shr(token) => Some(token.spans[0]),
        BinOp::AddAssign(token) => Some(token.spans[0]),
        BinOp::SubAssign(token) => Some(token.spans[0]),
        BinOp::MulAssign(token) => Some(token.spans[0]),
        BinOp::DivAssign(token) => Some(token.spans[0]),
        BinOp::RemAssign(token) => Some(token.spans[0]),
        BinOp::BitXorAssign(token) => Some(token.spans[0]),
        BinOp::BitAndAssign(token) => Some(token.spans[0]),
        BinOp::BitOrAssign(token) => Some(token.spans[0]),
        BinOp::ShlAssign(token) => Some(token.spans[0]),
        BinOp::ShrAssign(token) => Some(token.spans[0]),
        _ => None,
    }
}

/// Flags binary arithmetic operators outside the checked-arithmetic files.
pub(crate) fn on_binary(scanner: &mut Scanner<'_>, binary: &syn::ExprBinary) {
    if scanner.arith_allowed() == Answer::Yes {
        return;
    }
    if let Some(span) = binop_span(&binary.op) {
        scanner.report(
            span,
            rules::ARITH_OPERATOR,
            &Message::of(
                "arithmetic operator outside the checked-arithmetic module; use checked operations",
            ),
        );
    }
}

/// Flags unary minus on a non-literal outside the checked-arithmetic files.
pub(crate) fn on_unary(scanner: &mut Scanner<'_>, unary: &syn::ExprUnary) {
    if scanner.arith_allowed() == Answer::Yes {
        return;
    }
    if let UnOp::Neg(token) = &unary.op {
        if !matches!(unary.expr.as_ref(), Expr::Lit(_)) {
            scanner.report(
                token.spans[0],
                rules::ARITH_OPERATOR,
                &Message::of("unary minus on a non-literal; use checked negation"),
            );
        }
    }
}

/// Flags `as` casts to a numeric type (or to `_`).
pub(crate) fn on_cast(scanner: &mut Scanner<'_>, cast: &syn::ExprCast) {
    let numeric = match cast.ty.as_ref() {
        Type::Infer(_) => true,
        Type::Path(type_path) if type_path.qself.is_none() => {
            match types::primitive_of(scanner, &type_path.path) {
                Some(PrimitiveKind::Integer | PrimitiveKind::Float) => true,
                Some(PrimitiveKind::Bool | PrimitiveKind::Char | PrimitiveKind::Text) | None => {
                    false
                }
            }
        }
        _ => false,
    };
    if numeric {
        scanner.report(
            cast.as_token.span,
            rules::NUMERIC_CAST,
            &Message::of("`as` cast to a numeric type; use TryFrom or a checked conversion"),
        );
    }
}

/// Flags `let _ = ..` discards and refutable patterns in `let ... else`.
pub(crate) fn on_local(scanner: &mut Scanner<'_>, local: &syn::Local) {
    let wild = match &local.pat {
        Pat::Wild(_) => true,
        Pat::Type(typed) => matches!(typed.pat.as_ref(), Pat::Wild(_)),
        _ => false,
    };
    if wild {
        scanner.report(
            local.let_token.span,
            rules::DISCARDED_VALUE,
            &Message::of("`let _ = ..` discards a value; handle it"),
        );
    }
    if let Some(init) = &local.init {
        if init.diverge.is_some() {
            patterns::check_refutable(scanner, &local.pat, local.let_token.span);
        }
    }
}

/// Is `expr` a call of a path ending in `default` with no arguments, such as
/// `Default::default()`, `Pair::default()` or `<Pair as Default>::default()`?
#[must_use]
fn is_default_call(expr: &Expr) -> Answer {
    match expr {
        Expr::Call(call) if call.args.is_empty() => match &*call.func {
            Expr::Path(path) => path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "default")
                .answer(),
            _ => Answer::No,
        },
        Expr::Paren(inner) => is_default_call(&inner.expr),
        Expr::Group(inner) => is_default_call(&inner.expr),
        _ => Answer::No,
    }
}

/// Flags a struct literal on a workspace struct that takes the rest of its
/// fields from a `default()` call (`Pair { a, ..Default::default() }`).
pub(crate) fn on_struct_update(scanner: &mut Scanner<'_>, literal: &syn::ExprStruct) {
    let (Some(dots), Some(base)) = (&literal.dot2_token, &literal.rest) else {
        return;
    };
    if is_default_call(base) == Answer::No {
        return;
    }
    let resolved = types::resolve_path(scanner, &literal.path);
    match resolved.kind {
        Kind::Workspace | Kind::Local => scanner.report(
            dots.spans[0],
            rules::DEFAULT_CONSTRUCTION,
            &Message::of(
                "struct update from `default()` hides which fields were chosen; list every field",
            ),
        ),
        Kind::Ambiguous | Kind::External | Kind::Unresolved | Kind::Variant => {}
    }
}

/// Does `expr` produce a fresh value directly: a call, method call, macro call
/// or `?` expression, possibly in parentheses?
#[must_use]
fn is_fresh_value(expr: &Expr) -> Answer {
    match expr {
        Expr::Call(_) | Expr::MethodCall(_) | Expr::Macro(_) | Expr::Try(_) => Answer::Yes,
        Expr::Paren(inner) => is_fresh_value(&inner.expr),
        Expr::Group(inner) => is_fresh_value(&inner.expr),
        _ => Answer::No,
    }
}

/// Flags `drop(<call>)`: the value just produced is thrown away. `drop` is
/// resolved through `use` and aliases; a workspace item named `drop` is not it.
pub(crate) fn on_drop_call(scanner: &mut Scanner<'_>, call: &syn::ExprCall, path: &syn::ExprPath) {
    let [argument] = call.args.iter().collect::<Vec<_>>()[..] else {
        return;
    };
    if is_fresh_value(argument) == Answer::No {
        return;
    }
    let resolved = types::resolve_path(scanner, &path.path);
    match resolved.kind {
        Kind::Local | Kind::Variant | Kind::Workspace => return,
        Kind::Ambiguous | Kind::External | Kind::Unresolved => {}
    }
    if matches!(resolved.text().as_str(), "drop" | "std::mem::drop") {
        scanner.report(
            paths::first_span(&path.path),
            rules::DROPPED_CALL_RESULT,
            &Message::of(
                "the freshly produced value is discarded; handle it, or bind it and use it",
            ),
        );
    }
}

/// Does a token stream mention `name` as an identifier, or inside a literal (an
/// inline format argument such as `"{_x}"`)?
#[must_use]
fn tokens_mention(tokens: &proc_macro2::TokenStream, name: &Name) -> Answer {
    tokens
        .clone()
        .into_iter()
        .any(|tree| match tree {
            proc_macro2::TokenTree::Ident(ident) => ident == name.as_str(),
            proc_macro2::TokenTree::Literal(literal) => literal.to_string().contains(name.as_str()),
            proc_macro2::TokenTree::Group(group) => {
                tokens_mention(&group.stream(), name) == Answer::Yes
            }
            proc_macro2::TokenTree::Punct(_) => false,
        })
        .answer()
}

/// Collects the `_`-prefixed identifiers (not bare `_`) a pattern binds.
fn underscore_names(pat: &Pat, out: &mut Vec<syn::Ident>) {
    match pat {
        Pat::Ident(ident) => {
            let text = ident.ident.to_string();
            if text.starts_with('_') && text != "_" {
                out.push(ident.ident.clone());
            }
            if let Some((_, sub)) = &ident.subpat {
                underscore_names(sub, out);
            }
        }
        Pat::Tuple(tuple) => {
            for elem in &tuple.elems {
                underscore_names(elem, out);
            }
        }
        Pat::TupleStruct(tuple_struct) => {
            for elem in &tuple_struct.elems {
                underscore_names(elem, out);
            }
        }
        Pat::Slice(slice) => {
            for elem in &slice.elems {
                underscore_names(elem, out);
            }
        }
        Pat::Struct(pat_struct) => {
            for field in &pat_struct.fields {
                underscore_names(&field.pat, out);
            }
        }
        Pat::Or(or) => {
            for case in &or.cases {
                underscore_names(case, out);
            }
        }
        Pat::Paren(inner) => underscore_names(&inner.pat, out),
        Pat::Reference(inner) => underscore_names(&inner.pat, out),
        Pat::Type(inner) => underscore_names(&inner.pat, out),
        _ => {}
    }
}

/// Flags a `let` binding of an `_`-prefixed name that nothing later in the
/// block mentions. Any later occurrence of the identifier in the statements that
/// follow counts as a read (closures, nested blocks, macro tokens, inline format
/// arguments, a shadowing `let`), so the rule never reports on a doubtful case.
pub(crate) fn check_underscore_bindings(scanner: &mut Scanner<'_>, stmts: &[syn::Stmt]) {
    for (index, stmt) in stmts.iter().enumerate() {
        let syn::Stmt::Local(local) = stmt else {
            continue;
        };
        let mut names = Vec::new();
        underscore_names(&local.pat, &mut names);
        for name in names {
            let text = Name::from_string(name.to_string());
            let mut finder = Occurrence {
                name: &text,
                seen: Answer::No,
            };
            for later in stmts.iter().skip(index).skip(1) {
                finder.visit_stmt(later);
            }
            if finder.seen == Answer::No {
                scanner.report(
                    name.span(),
                    rules::UNUSED_UNDERSCORE_BINDING,
                    &Message::from_string(format!(
                        "`{}` is bound and never read, which discards the value; bind and read it, handle the error, or (for a guard that must live to the end of the scope) use a closure-scoped helper or an explicitly named binding that is `drop`ped at the end",
                        text.as_str()
                    )),
                );
            }
        }
    }
}

/// Flags `_ = ..` assignments that discard a value.
pub(crate) fn on_assign(scanner: &mut Scanner<'_>, assign: &syn::ExprAssign) {
    if let Expr::Infer(infer) = assign.left.as_ref() {
        scanner.report(
            infer.underscore_token.span,
            rules::DISCARDED_VALUE,
            &Message::of("`_ = ..` discards a value; handle it"),
        );
    }
}

/// Checks the pattern of an `if let` or `while let` expression for refutability rules.
pub(crate) fn on_let_expr(scanner: &mut Scanner<'_>, expr_let: &syn::ExprLet) {
    patterns::check_refutable(scanner, &expr_let.pat, expr_let.let_token.span);
}

/// Flags closure parameters annotated with a `bool` type.
pub(crate) fn on_closure(scanner: &mut Scanner<'_>, closure: &syn::ExprClosure) {
    for input in &closure.inputs {
        if let Pat::Type(typed) = input {
            let mut kinds = Vec::new();
            types::collect_primitives(scanner, &typed.ty, &mut kinds);
            if kinds.contains(&PrimitiveKind::Bool) {
                scanner.report(
                    closure.or1_token.spans[0],
                    rules::BOOL_PARAM,
                    &Message::of("closure takes a bool parameter; use a two-variant enum"),
                );
            }
        }
    }
}

/// Flags float literals, including integer literals with an `f32` or `f64` suffix.
pub(crate) fn on_lit(scanner: &mut Scanner<'_>, lit: &Lit) {
    let float = match lit {
        Lit::Float(_) => true,
        Lit::Int(int) => matches!(int.suffix(), "f32" | "f64"),
        _ => false,
    };
    if float {
        scanner.report(lit.span(), rules::FLOAT_TYPE, &Message::of("float literal"));
    }
}

/// Applies the path rules to a type path and flags float types.
pub(crate) fn on_type_path(scanner: &mut Scanner<'_>, type_path: &syn::TypePath) {
    paths::check_path(
        scanner,
        &type_path.path,
        qself_of(type_path.qself.as_ref()),
        PathRole::Type,
    );
}

/// Does a `return` operand call a path whose last segment is `Err`? Such a
/// `return` fails the test, so it is not an early exit.
#[must_use]
pub(crate) fn is_err_call(operand: Option<&Expr>) -> Answer {
    match operand {
        Some(Expr::Call(call)) => match &*call.func {
            Expr::Path(path) => path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "Err")
                .answer(),
            _ => Answer::No,
        },
        _ => Answer::No,
    }
}

/// Flags a `return` in a test fn's own body that comes before its first
/// assertion macro; closures, async blocks and nested fns have their own body.
/// A `return` of an `Err(..)` call (`fails`) is exempt: it fails the test.
pub(crate) fn on_return(scanner: &mut Scanner<'_>, span: Span, fails: Answer) {
    if fails == Answer::No
        && scanner.bodies.last() == Some(&Body::Test)
        && scanner.assertions.last() == Some(&AssertionSeen::No)
    {
        scanner.report(
            span,
            rules::TEST_EARLY_RETURN,
            &Message::of(EARLY_RETURN_MESSAGE),
        );
    }
}

/// Visits a `let .. else` and reports one `test-early-return` finding at its
/// `else` in place of the findings for the `return`s inside the else block.
pub(crate) fn visit_let_else<'ast>(
    scanner: &mut Scanner<'_>,
    init: &'ast syn::LocalInit,
    diverge: &'ast (Else, Box<Expr>),
) {
    scanner.visit_expr(&init.expr);
    let before = scanner.findings.len();
    scanner.visit_expr(&diverge.1);
    let tail = scanner.findings.split_off(before);
    let rule = Name::of(rules::TEST_EARLY_RETURN.name());
    let (early, rest): (Vec<Finding>, Vec<Finding>) =
        tail.into_iter().partition(|found| found.rule == rule);
    scanner.findings.extend(rest);
    if !early.is_empty() {
        scanner.report(
            diverge.0.span,
            rules::TEST_EARLY_RETURN,
            &Message::of(EARLY_RETURN_MESSAGE),
        );
    }
}

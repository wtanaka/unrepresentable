//! The `syn::visit::Visit` implementation: thin dispatch to the rule helpers.
use core::mem;

use syn::visit::{self, Visit};

use syn::{Attribute, ImplItem, Item, Stmt, TraitItem};

use super::exprs;

use super::items;

use super::macros;

use super::paths;

use super::paths::{qself_of, PathRole, Qself};

use super::patterns;

use super::{AssertionSeen, Body, Scanner};

use crate::loader::Scope;

use crate::rules;

use crate::types::{Answer, Col, Line, Message, Name, Text};

impl<'ast> Visit<'ast> for Scanner<'_> {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        items::on_attribute(self, attr);
        visit::visit_attribute(self, attr);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.blocks.push(items::block_scope(&block.stmts));
        exprs::check_underscore_bindings(self, &block.stmts);
        visit::visit_block(self, block);
        match self.blocks.pop() {
            Some(_) | None => {}
        }
    }

    fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
        exprs::on_assign(self, assign);
        visit::visit_expr_assign(self, assign);
    }

    fn visit_expr_async(&mut self, block: &'ast syn::ExprAsync) {
        self.bodies.push(Body::Other);
        visit::visit_expr_async(self, block);
        self.leave_body();
    }

    fn visit_expr_binary(&mut self, binary: &'ast syn::ExprBinary) {
        exprs::on_binary(self, binary);
        visit::visit_expr_binary(self, binary);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = call.func.as_ref() {
            paths::check_path(
                self,
                &path.path,
                qself_of(path.qself.as_ref()),
                PathRole::Callee,
            );
            exprs::on_drop_call(self, call, path);
            for attr in &call.attrs {
                self.visit_attribute(attr);
            }
            visit::visit_expr_path(self, path);
            for arg in &call.args {
                self.visit_expr(arg);
            }
        } else {
            visit::visit_expr_call(self, call);
        }
    }

    fn visit_expr_cast(&mut self, cast: &'ast syn::ExprCast) {
        exprs::on_cast(self, cast);
        visit::visit_expr_cast(self, cast);
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        exprs::on_closure(self, closure);
        self.bodies.push(Body::Other);
        visit::visit_expr_closure(self, closure);
        self.leave_body();
    }

    fn visit_expr_if(&mut self, expr: &'ast syn::ExprIf) {
        if expr.else_branch.is_none() {
            patterns::check_swallowed_let(self, &expr.cond);
        }
        visit::visit_expr_if(self, expr);
    }

    fn visit_expr_let(&mut self, expr: &'ast syn::ExprLet) {
        exprs::on_let_expr(self, expr);
        visit::visit_expr_let(self, expr);
    }

    fn visit_expr_match(&mut self, expr: &'ast syn::ExprMatch) {
        patterns::check_match_arms(self, &expr.arms);
        patterns::check_swallowed_arms(self, &expr.arms);
        visit::visit_expr_match(self, expr);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        paths::check_method_name(
            self,
            call.method.span(),
            &Name::from_string(call.method.to_string()),
        );
        paths::check_leak_method(self, call);
        paths::check_probe_method(self, call);
        visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        paths::check_path(
            self,
            &path.path,
            qself_of(path.qself.as_ref()),
            PathRole::Expr,
        );
        visit::visit_expr_path(self, path);
    }

    fn visit_expr_return(&mut self, expr: &'ast syn::ExprReturn) {
        exprs::on_return(
            self,
            expr.return_token.span,
            exprs::is_err_call(expr.expr.as_deref()),
        );
        visit::visit_expr_return(self, expr);
    }

    fn visit_expr_struct(&mut self, expr: &'ast syn::ExprStruct) {
        paths::check_path(
            self,
            &expr.path,
            qself_of(expr.qself.as_ref()),
            PathRole::Type,
        );
        exprs::on_struct_update(self, expr);
        visit::visit_expr_struct(self, expr);
    }

    fn visit_expr_unary(&mut self, unary: &'ast syn::ExprUnary) {
        exprs::on_unary(self, unary);
        visit::visit_expr_unary(self, unary);
    }

    fn visit_expr_while(&mut self, expr: &'ast syn::ExprWhile) {
        patterns::check_swallowed_let(self, &expr.cond);
        visit::visit_expr_while(self, expr);
    }

    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        self.with_attr_scope(impl_item_attrs(item), |scanner| {
            visit::visit_impl_item(scanner, item);
        });
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        in_fn(self, &item.sig.ident, &item.attrs, |scanner| {
            items::on_signature(scanner, &item.sig, &item.attrs, &item.vis);
            visit::visit_impl_item_fn(scanner, item);
        });
    }

    fn visit_item(&mut self, item: &'ast Item) {
        self.with_attr_scope(item_attrs(item), |scanner| visit::visit_item(scanner, item));
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.push_item(Text::from_string(format!("const {}", item.ident)));
        visit::visit_item_const(self, item);
        self.pop_item();
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.push_item(Text::from_string(format!("enum {}", item.ident)));
        for variant in &item.variants {
            self.push_item(Text::from_string(format!("variant {}", variant.ident)));
            items::on_named_fields(self, &variant.fields);
            self.pop_item();
        }
        visit::visit_item_enum(self, item);
        self.pop_item();
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        in_fn(self, &item.sig.ident, &item.attrs, |scanner| {
            items::on_signature(scanner, &item.sig, &item.attrs, &item.vis);
            visit::visit_item_fn(scanner, item);
        });
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let new_ctx = items::impl_ctx_for(self, item);
        let new_self = items::self_path_of(self, item);
        let saved_ctx = mem::replace(&mut self.impl_ctx, new_ctx);
        let saved_self = mem::replace(&mut self.self_ty, new_self);
        self.push_item(items::impl_label(item));
        items::on_impl_traits(self, item);
        visit::visit_item_impl(self, item);
        self.pop_item();
        self.impl_ctx = saved_ctx;
        self.self_ty = saved_self;
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        match &item.ident {
            Some(name) => {
                for attr in &item.attrs {
                    self.visit_attribute(attr);
                }
                self.push_item(Text::from_string(format!("macro {name}!")));
                macros::scan_macro_rules(
                    self,
                    item.mac.tokens.clone(),
                    paths::first_span(&item.mac.path),
                );
                self.pop_item();
            }
            None => visit::visit_item_macro(self, item),
        }
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let name = Name::from_string(item.ident.to_string());
        self.module.push(name);
        let saved_items = mem::take(&mut self.items);
        if item.content.is_some() {
            visit::visit_item_mod(self, item);
        } else {
            for attr in &item.attrs {
                self.visit_attribute(attr);
            }
            enter_file_module(self, item);
        }
        self.items = saved_items;
        match self.module.pop() {
            Some(_) | None => {}
        }
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.push_item(Text::from_string(format!("static {}", item.ident)));
        visit::visit_item_static(self, item);
        self.pop_item();
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.push_item(Text::from_string(format!("struct {}", item.ident)));
        items::on_default_derive(self, &item.attrs);
        items::on_named_fields(self, &item.fields);
        visit::visit_item_struct(self, item);
        self.pop_item();
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.push_item(Text::from_string(format!("trait {}", item.ident)));
        visit::visit_item_trait(self, item);
        self.pop_item();
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.push_item(Text::from_string(format!("type {}", item.ident)));
        visit::visit_item_type(self, item);
        self.pop_item();
    }

    fn visit_item_union(&mut self, item: &'ast syn::ItemUnion) {
        self.push_item(Text::from_string(format!("union {}", item.ident)));
        items::on_default_derive(self, &item.attrs);
        items::on_field_list(self, item.fields.named.iter());
        visit::visit_item_union(self, item);
        self.pop_item();
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        items::on_use_tree(self, &item.tree);
        visit::visit_item_use(self, item);
    }

    fn visit_lit(&mut self, lit: &'ast syn::Lit) {
        exprs::on_lit(self, lit);
        visit::visit_lit(self, lit);
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        exprs::on_local(self, local);
        match &local.init {
            Some(init) => match &init.diverge {
                Some(diverge) => {
                    for attr in &local.attrs {
                        self.visit_attribute(attr);
                    }
                    self.visit_pat(&local.pat);
                    exprs::visit_let_else(self, init, diverge);
                }
                None => visit::visit_local(self, local),
            },
            None => visit::visit_local(self, local),
        }
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        macros::on_macro(self, mac);
    }

    fn visit_pat_struct(&mut self, pat: &'ast syn::PatStruct) {
        paths::check_path(self, &pat.path, qself_of(pat.qself.as_ref()), PathRole::Pat);
        patterns::check_struct_rest(self, pat);
        visit::visit_pat_struct(self, pat);
    }

    fn visit_pat_tuple_struct(&mut self, pat: &'ast syn::PatTupleStruct) {
        paths::check_path(self, &pat.path, qself_of(pat.qself.as_ref()), PathRole::Pat);
        patterns::check_tuple_struct_rest(self, pat);
        visit::visit_pat_tuple_struct(self, pat);
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        match stmt {
            Stmt::Local(local) => self.with_attr_scope(&local.attrs, |scanner| {
                visit::visit_stmt(scanner, stmt);
            }),
            Stmt::Macro(mac) => self.with_attr_scope(&mac.attrs, |scanner| {
                visit::visit_stmt(scanner, stmt);
            }),
            _ => visit::visit_stmt(self, stmt),
        }
    }

    fn visit_trait_bound(&mut self, bound: &'ast syn::TraitBound) {
        paths::check_path(self, &bound.path, Qself::Absent, PathRole::Type);
        visit::visit_trait_bound(self, bound);
    }

    fn visit_trait_item(&mut self, item: &'ast TraitItem) {
        self.with_attr_scope(trait_item_attrs(item), |scanner| {
            visit::visit_trait_item(scanner, item);
        });
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        in_fn(self, &item.sig.ident, &item.attrs, |scanner| {
            let saved = mem::take(&mut scanner.impl_ctx);
            items::on_signature(scanner, &item.sig, &item.attrs, &syn::Visibility::Inherited);
            scanner.impl_ctx = saved;
            visit::visit_trait_item_fn(scanner, item);
        });
    }

    fn visit_type_path(&mut self, type_path: &'ast syn::TypePath) {
        exprs::on_type_path(self, type_path);
        visit::visit_type_path(self, type_path);
    }
}

/// Returns the outer attributes of an item.
fn item_attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(inner) => &inner.attrs,
        Item::Enum(inner) => &inner.attrs,
        Item::ExternCrate(inner) => &inner.attrs,
        Item::Fn(inner) => &inner.attrs,
        Item::ForeignMod(inner) => &inner.attrs,
        Item::Impl(inner) => &inner.attrs,
        Item::Macro(inner) => &inner.attrs,
        Item::Mod(inner) => &inner.attrs,
        Item::Static(inner) => &inner.attrs,
        Item::Struct(inner) => &inner.attrs,
        Item::Trait(inner) => &inner.attrs,
        Item::TraitAlias(inner) => &inner.attrs,
        Item::Type(inner) => &inner.attrs,
        Item::Union(inner) => &inner.attrs,
        Item::Use(inner) => &inner.attrs,
        _ => &[],
    }
}

/// Returns the outer attributes of an `impl` item.
fn impl_item_attrs(item: &ImplItem) -> &[Attribute] {
    match item {
        ImplItem::Const(inner) => &inner.attrs,
        ImplItem::Fn(inner) => &inner.attrs,
        ImplItem::Type(inner) => &inner.attrs,
        ImplItem::Macro(inner) => &inner.attrs,
        _ => &[],
    }
}

/// Returns the outer attributes of a trait item.
fn trait_item_attrs(item: &TraitItem) -> &[Attribute] {
    match item {
        TraitItem::Const(inner) => &inner.attrs,
        TraitItem::Fn(inner) => &inner.attrs,
        TraitItem::Type(inner) => &inner.attrs,
        TraitItem::Macro(inner) => &inner.attrs,
        _ => &[],
    }
}

/// Runs `body` as a fn: item label, test scope, assertion tracking.
fn in_fn(
    scanner: &mut Scanner<'_>,
    name: &syn::Ident,
    attrs_list: &[Attribute],
    body: impl FnOnce(&mut Scanner<'_>),
) {
    let is_test = items::is_test_fn(scanner, attrs_list);
    let saved = scanner.scope;
    if is_test == Answer::Yes {
        scanner.scope = Scope::Test;
        scanner.assertions.push(AssertionSeen::No);
        scanner.bodies.push(Body::Test);
    } else {
        scanner.bodies.push(Body::Other);
    }
    scanner.push_item(Text::from_string(format!("fn {name}")));
    body(scanner);
    scanner.leave_body();
    if is_test == Answer::Yes && scanner.assertions.pop() == Some(AssertionSeen::No) {
        scanner.report(
            name.span(),
            rules::TEST_ASSERTION,
            &Message::from_string(format!("test fn `{name}` contains no assertion macro")),
        );
    }
    scanner.pop_item();
    scanner.scope = saved;
}

/// Loads and scans the file behind a `mod x;` declaration.
fn enter_file_module(scanner: &mut Scanner<'_>, item: &syn::ItemMod) {
    let start = item.ident.span().start();
    let key = (Line::new(start.line), Col::new(start.column));
    let declared = scanner
        .file
        .mods
        .iter()
        .find(|candidate| (candidate.line, candidate.col) == key);
    let loaded = declared.and_then(|found| scanner.files.get(found.file.index().get()));
    let Some(child) = loaded else {
        scanner.report(
            item.ident.span(),
            rules::UNSCANNABLE_INCLUDE,
            &Message::of("module declared where its file cannot be followed (inside a macro body)"),
        );
        return;
    };
    let saved_file = mem::replace(&mut scanner.file, child);
    let saved_path = mem::replace(&mut scanner.path, child.path.clone());
    let saved_source = mem::replace(&mut scanner.source, &child.source);
    let saved_scope = scanner.scope;
    if child.scope == Scope::Test {
        scanner.scope = Scope::Test;
    }
    let saved_blocks = mem::take(&mut scanner.blocks);
    Visit::visit_file(scanner, &child.ast);
    scanner.blocks = saved_blocks;
    scanner.scope = saved_scope;
    scanner.source = saved_source;
    scanner.path = saved_path;
    scanner.file = saved_file;
}

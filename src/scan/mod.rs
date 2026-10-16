//! The rule scanner: walks every loaded file with `syn` and applies every
//! rule in every scope.
/// Expression-level rules.
mod exprs;

/// Attribute, signature and item-level rules.
mod items;

/// Macro invocation rules and macro body parsing.
mod macros;

/// Path, method-name and macro-name rules.
mod paths;

/// Pattern rules, such as refutable patterns and wildcards.
mod patterns;

/// Token-level scan used for macro bodies that do not parse as Rust.
mod tokens;

/// Placeholder substitution for `macro_rules!` transcribers.
mod substitute;

/// Type classification helpers: primitives, newtypes and path resolution.
mod types;

/// The `syn` visitor that drives the scan and dispatches to the rule modules.
mod visit;

use alloc::collections::BTreeSet;

use proc_macro2::Span;

use syn::visit::Visit as _;

use crate::config::{Config, MemberConfig};

use crate::findings::Finding;

use crate::index::{BlockScope, Resolver};

use crate::loader::{self, LoadedCrate, LoadedFile, Scope};

use crate::rules::{self, RuleId};

use crate::types::{Answer, Col, Count, Holds as _, Line, Message, Name, RelPath, Source, Text};

/// Immutable context shared by the whole scan of one crate.
pub(crate) struct Env<'ctx> {
    /// Type names that are banned when they appear as the leaf of a glob-imported path.
    pub banned_leaves: &'ctx BTreeSet<Name>,
    /// The loaded configuration for the whole workspace.
    pub config: &'ctx Config,
    /// The configuration of the workspace member being scanned.
    pub member: &'ctx MemberConfig,
    /// Names of the types registered as newtypes for this crate.
    pub newtypes: &'ctx BTreeSet<Name>,
    /// Resolver used to turn a written path into a resolved path.
    pub resolver: Resolver<'ctx>,
}

/// One `#[expect(..)]` lint use, to be matched against the ledger.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ExpectUse {
    /// Zero-based or one-based column of the `#[expect]` attribute, as reported.
    pub col: Col,
    /// Label of the item that carries the attribute.
    pub item: Text,
    /// Line of the `#[expect]` attribute.
    pub line: Line,
    /// Path text of the lint named inside the `#[expect(..)]`.
    pub lint: Text,
    /// File containing the attribute.
    pub path: RelPath,
}

/// Everything one crate scan produces: `#[expect]` uses and findings.
pub(crate) struct ScanOutput {
    /// Every `#[expect(..)]` lint use found, to be matched against the ledger.
    pub expects: Vec<ExpectUse>,
    /// Every rule violation found in the crate.
    pub findings: Vec<Finding>,
}

/// What kind of `impl` block, if any, encloses the code being scanned.
#[derive(Clone, Debug, Default)]
pub(crate) enum ImplCtx {
    /// Not inside any `impl` block.
    #[default]
    Free,
    /// An inherent `impl`; the name is set for a one-field tuple struct.
    Inherent(Option<Name>),
    /// Inside an `impl Trait for Type` block.
    TraitImpl,
}

/// Mutable state of one scan of a crate, threaded through the visitor.
pub(crate) struct Scanner<'ctx> {
    /// One entry per enclosing test fn: has an assertion macro been seen?
    pub assertions: Vec<AssertionSeen>,
    /// The block scopes (local `use` and items) enclosing the current position.
    pub blocks: Vec<BlockScope>,
    /// One entry per enclosing fn, closure or async block: whose body is this?
    pub bodies: Vec<Body>,
    /// Immutable context shared by the whole scan.
    pub env: &'ctx Env<'ctx>,
    /// `#[expect(..)]` uses collected so far.
    pub expects: Vec<ExpectUse>,
    /// The file currently being scanned.
    pub file: &'ctx LoadedFile,
    /// All files loaded for the crate, used to find `include!` targets.
    pub files: &'ctx [LoadedFile],
    /// Rule violations collected so far.
    pub findings: Vec<Finding>,
    /// The kind of `impl` block enclosing the current position.
    pub impl_ctx: ImplCtx,
    /// How many nested `include!` expansions are currently being scanned.
    pub include_depth: Count,
    /// Labels of the enclosing items, used to describe where a finding sits.
    pub items: Vec<Text>,
    /// Names of the enclosing inline modules.
    pub module: Vec<Name>,
    /// Path of the file or included file currently being scanned.
    pub path: RelPath,
    /// The places of the `..` of enum-variant patterns that `refutable-shortcut`
    /// already reports as part of its pattern, so `struct-rest-pattern` stays
    /// silent there: file, line and column.
    pub refutable_rests: Vec<(RelPath, Line, Col)>,
    /// The scope (production, test, example, ...) of the current position.
    pub scope: Scope,
    /// Resolved path of the enclosing `impl` block's self type, if it is a plain path.
    pub self_ty: Option<Vec<Name>>,
    /// Source text of the file or included file currently being scanned.
    pub source: &'ctx Source,
}

/// Whose body the innermost enclosing fn, closure or async block is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Body {
    /// A closure, an async block or a fn that is not a test: a `return` here
    /// does not end a test fn.
    Other,
    /// A test fn: a `return` here ends the test.
    Test,
}

/// Whether an assertion macro has been seen in a test fn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssertionSeen {
    /// No assertion macro has been seen yet.
    No,
    /// At least one assertion macro has been seen.
    Yes,
}

impl Scanner<'_> {
    /// Is the current file listed as a checked-arithmetic file?
    pub(crate) fn arith_allowed(&self) -> Answer {
        self.env
            .config
            .arith_files
            .iter()
            .any(|glob| glob.matches(&self.path) == Answer::Yes)
            .answer()
    }

    /// Describes the enclosing modules and items as one `/`-separated string.
    pub(crate) fn item_text(&self) -> Text {
        let mut parts: Vec<String> = self
            .module
            .iter()
            .map(|name| format!("mod {}", name.as_str()))
            .collect();
        parts.extend(self.items.iter().map(|label| label.as_str().to_owned()));
        Text::from_string(parts.join(" / "))
    }

    /// Leaves the innermost enclosing fn, closure or async block.
    pub(crate) fn leave_body(&mut self) {
        match self.bodies.pop() {
            Some(_) | None => {}
        }
    }

    /// Leaves the innermost enclosing item label.
    pub(crate) fn pop_item(&mut self) {
        match self.items.pop() {
            Some(_) | None => {}
        }
    }

    /// Enters an item with the given label.
    pub(crate) fn push_item(&mut self, label: Text) {
        self.items.push(label);
    }

    /// Records a finding for a built-in rule at `span`.
    pub(crate) fn report(&mut self, span: Span, rule: RuleId, message: &Message) {
        self.report_named(span, &Name::of(rule.name()), message);
    }

    /// Records a finding for the ban set `id` at `span`.
    pub(crate) fn report_banned(&mut self, span: Span, id: &Name, message: &Message) {
        let rule = Name::from_string(format!("{}:{}", rules::BANNED.name(), id.as_str()));
        self.report_named(span, &rule, message);
    }

    /// Records a finding under the rule named `rule` at `span`.
    pub(crate) fn report_named(&mut self, span: Span, rule: &Name, message: &Message) {
        let (line, col) = span_line_col(span);
        let item = self.item_text();
        self.findings.push(Finding {
            col,
            item,
            line,
            message: message.clone(),
            path: self.path.clone(),
            rule: rule.clone(),
        });
    }

    /// Applies `cfg(test)` from `attrs_list` to the scope for the duration of `inner`.
    pub(crate) fn with_attr_scope<R>(
        &mut self,
        attrs_list: &[syn::Attribute],
        inner: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let saved = self.scope;
        if loader::has_cfg_test(attrs_list) == Answer::Yes {
            self.scope = Scope::Test;
        }
        let result = inner(self);
        self.scope = saved;
        result
    }
}

/// Scans every file of `krate`.
pub(crate) fn scan_crate<'ctx>(env: &'ctx Env<'ctx>, krate: &'ctx LoadedCrate) -> ScanOutput {
    let mut findings: Vec<Finding> = krate.issues.clone();
    let mut expects = Vec::new();
    if let Some(root) = krate.files.first() {
        let mut scanner = Scanner {
            assertions: Vec::new(),
            blocks: Vec::new(),
            bodies: Vec::new(),
            env,
            expects: Vec::new(),
            file: root,
            files: &krate.files,
            findings: Vec::new(),
            impl_ctx: ImplCtx::default(),
            include_depth: Count::new(0),
            items: Vec::new(),
            module: Vec::new(),
            path: root.path.clone(),
            refutable_rests: Vec::new(),
            scope: root.scope,
            self_ty: None,
            source: &root.source,
        };
        scanner.visit_file(&root.ast);
        findings.append(&mut scanner.findings);
        expects.append(&mut scanner.expects);
    }
    ScanOutput { expects, findings }
}

/// Converts a span to the one-based line and column used in findings.
pub(crate) fn span_line_col(span: Span) -> (Line, Col) {
    let start = span.start();
    let col = match Col::from_zero_based(Count::new(start.column)) {
        Ok(found) => found,
        Err(_) => Col::new(start.column),
    };
    (Line::new(start.line), col)
}

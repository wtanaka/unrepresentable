//! Loads every source file reachable from a cargo target: `mod x;` with
//! `#[path]`, `mod.rs`, `x.rs` + `x/`, inline modules, and `include!`.
use syn::visit::{self, Visit};

use syn::{Expr, ExprLit, Item, Lit, Meta};

use crate::attrs;

use crate::failure::Failure;

use crate::findings::Finding;

use crate::io::Root;

use crate::rules;

use crate::types::{Answer, Col, Count, Holds as _, Line, Message, Name, RelPath, Source, Text};

use crate::workspace::{Target, TargetKind};

/// The kind of code a file belongs to, which decides how strictly it is scanned.
///
/// Test scope is per item: a `#[test]` fn, a `#[cfg(test)]` item or module,
/// `all(test, ..)`, and everything under a member's `tests/` directory. It only
/// changes `abort-assert` (allowed in tests); `test-assertion` applies to test
/// fns. `cfg` is never evaluated: both sides of a `#[cfg(..)]` and every declared
/// `mod` are loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Scope {
    /// Code of a benchmark target.
    Bench,
    /// Code of a `build.rs` build script.
    BuildScript,
    /// Code of an example target.
    Example,
    /// Shipped library or binary code.
    Production,
    /// Test code, including `#[cfg(test)]` modules and files under `tests/`.
    Test,
}

impl Scope {
    /// The scope that files of a target of the given kind start in.
    pub(crate) const fn of_target(kind: TargetKind) -> Self {
        match kind {
            TargetKind::Lib | TargetKind::Bin => Self::Production,
            TargetKind::Example => Self::Example,
            TargetKind::Test => Self::Test,
            TargetKind::Bench => Self::Bench,
            TargetKind::Build => Self::BuildScript,
        }
    }
}

/// Index of a loaded file within its crate, in load order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileId(Count);

impl FileId {
    /// The position of the file in the crate's file list.
    pub(crate) const fn index(self) -> Count {
        self.0
    }
}

/// A `mod x;` declaration and the file it loaded.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ModDecl {
    /// Zero-based column of the module name.
    pub col: Col,
    /// The file that the declaration loaded.
    pub file: FileId,
    /// One-based line of the module name.
    pub line: Line,
}

/// An `include!("literal")` and the file it names.
#[derive(Clone, Debug)]
pub(crate) struct Included {
    /// Zero-based column of the `include!` macro.
    pub col: Col,
    /// One-based line of the `include!` macro.
    pub line: Line,
    /// Path of the included file, relative to the root.
    pub path: RelPath,
    /// Text of the included file.
    pub source: Source,
}

/// One parsed source file of a crate together with what it pulls in.
pub(crate) struct LoadedFile {
    /// The parsed syntax tree of the file.
    pub ast: syn::File,
    /// The `include!` files referenced from this file.
    pub includes: Vec<Included>,
    /// The `mod x;` declarations in this file and the files they loaded.
    pub mods: Vec<ModDecl>,
    /// Path of the file, relative to the root.
    pub path: RelPath,
    /// The scope this file is scanned under.
    pub scope: Scope,
    /// The raw text of the file.
    pub source: Source,
}

/// All files reachable from one cargo target.
pub(crate) struct LoadedCrate {
    /// The loaded files; the crate root is always first.
    pub files: Vec<LoadedFile>,
    /// Findings raised while loading, such as unscannable includes.
    pub issues: Vec<Finding>,
    /// The kind of cargo target this crate was loaded for.
    pub kind: TargetKind,
}

/// Where the file being walked sits, used to resolve its child modules.
#[derive(Clone, Debug)]
struct Position {
    /// Directory containing the file.
    file_dir: RelPath,
    /// Whether the file is a `mod.rs`-style file or crate root.
    mod_rs: ModRs,
    /// File name without the `.rs` extension.
    stem: Name,
}

/// Whether a file owns its directory, as `mod.rs` and crate roots do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModRs {
    /// The file is `x.rs`; its children live in the `x/` directory.
    No,
    /// The file is `mod.rs` or a crate root; its children live beside it.
    Yes,
}

/// Working state while loading the files of one crate.
struct Loader<'ctx> {
    /// Files loaded so far, in load order.
    files: Vec<LoadedFile>,
    /// Findings raised so far while loading.
    issues: Vec<Finding>,
    /// Directory of the workspace member, relative to the root.
    member_dir: RelPath,
    /// The root used to read files.
    root: &'ctx Root,
}

/// The inline modules entered so far and the `mod x;` declarations met.
struct Trail {
    /// Names of the inline modules currently entered.
    inline: Vec<Name>,
    /// The `mod x;` declarations met so far in the current file.
    mods: Vec<ModDecl>,
}

impl Loader<'_> {
    /// Finds and reads the `include!` files of a parsed file, reporting non-literal ones.
    fn find_includes(
        &mut self,
        path: &RelPath,
        ast: &syn::File,
        position: &Position,
    ) -> Result<Vec<Included>, Failure> {
        let mut finder = IncludeFinder { found: Vec::new() };
        finder.visit_file(ast);
        let mut includes = Vec::new();
        for site in finder.found {
            let (line, col) = (site.line, site.col);
            match site.literal {
                Some(text) => {
                    let target = join_relative(&position.file_dir, &text);
                    let source = self.root.read(&target)?;
                    includes.push(Included {
                        col,
                        line,
                        path: target,
                        source,
                    });
                }
                None => self.issues.push(Finding {
                    col: one_based(col),
                    item: Text::from_string(String::new()),
                    line,
                    message: Message::of("include! with a non-literal argument cannot be scanned"),
                    path: path.clone(),
                    rule: Name::of(rules::UNSCANNABLE_INCLUDE.name()),
                }),
            }
        }
        Ok(includes)
    }

    /// Reads and parses a file, recursively loads its modules and returns its id.
    fn load_file(
        &mut self,
        path: &RelPath,
        scope: Scope,
        mod_rs: ModRs,
    ) -> Result<FileId, Failure> {
        let source = self.root.read(path)?;
        let ast = syn::parse_file(source.as_str()).map_err(|error| parse_failure(path, &error))?;
        let id = FileId(Count::new(self.files.len()));
        let under_tests = {
            let member_prefix = if self.member_dir.as_str().is_empty() {
                String::new()
            } else {
                format!("{}/", self.member_dir.as_str())
            };
            path.as_str()
                .strip_prefix(member_prefix.as_str())
                .is_some_and(|rest| rest.starts_with("tests/"))
        };
        let effective_scope = if under_tests { Scope::Test } else { scope };
        self.files.push(LoadedFile {
            ast: syn::File {
                shebang: None,
                attrs: Vec::new(),
                items: Vec::new(),
            },
            includes: Vec::new(),
            mods: Vec::new(),
            path: path.clone(),
            scope: effective_scope,
            source,
        });
        let position = Position {
            file_dir: path.parent(),
            mod_rs,
            stem: Name::of(path.file_name().as_str().trim_end_matches(".rs")),
        };
        let mut trail = Trail {
            inline: Vec::new(),
            mods: Vec::new(),
        };
        self.walk(&ast.items, &position, effective_scope, &mut trail)?;
        let mods = trail.mods;
        let includes = self.find_includes(path, &ast, &position)?;
        if let Some(slot) = self.files.get_mut(id.index().get()) {
            slot.ast = ast;
            slot.mods = mods;
            slot.includes = includes;
        }
        Ok(id)
    }

    /// Finds the file for a `mod x;` declaration, honouring `#[path]` and both layouts.
    fn resolve_mod(
        &self,
        module: &syn::ItemMod,
        position: &Position,
        inline: &[Name],
    ) -> Result<(RelPath, ModRs), Failure> {
        let mut dir = position.file_dir.clone();
        if position.mod_rs == ModRs::No {
            dir = dir.join(position.stem.as_str());
        }
        for part in inline {
            dir = dir.join(part.as_str());
        }
        if let Some(explicit) = mod_path_attr(&module.attrs) {
            let base = if inline.is_empty() {
                position.file_dir.clone()
            } else {
                dir
            };
            let joined = join_relative(&base, &explicit);
            return Ok((joined, ModRs::Yes));
        }
        let name = module.ident.to_string();
        let flat = dir.join(&format!("{name}.rs"));
        let nested = dir.join(&format!("{name}/mod.rs"));
        match (self.root.is_file(&flat), self.root.is_file(&nested)) {
            (Answer::Yes, Answer::No) => Ok((flat, ModRs::No)),
            (Answer::No, Answer::Yes) => Ok((nested, ModRs::Yes)),
            (Answer::Yes, Answer::Yes) => Err(Failure::parse_at(
                &position.file_dir,
                Line::new(module.ident.span().start().line),
                &Message::from_string(format!(
                    "module `{name}` has both {} and {}",
                    flat.as_str(),
                    nested.as_str()
                )),
            )),
            (Answer::No, Answer::No) => Err(Failure::parse_at(
                &position.file_dir,
                Line::new(module.ident.span().start().line),
                &Message::from_string(format!(
                    "module `{name}` not found (looked for {} and {})",
                    flat.as_str(),
                    nested.as_str()
                )),
            )),
        }
    }

    /// Walks items, descending into inline modules and loading out-of-line ones.
    fn walk(
        &mut self,
        items: &[Item],
        position: &Position,
        scope: Scope,
        trail: &mut Trail,
    ) -> Result<(), Failure> {
        for item in items {
            let Item::Mod(module) = item else {
                continue;
            };
            let inner_scope = if has_cfg_test(&module.attrs) == Answer::Yes {
                Scope::Test
            } else {
                scope
            };
            if let Some((_, inner_items)) = &module.content {
                trail
                    .inline
                    .push(Name::from_string(module.ident.to_string()));
                self.walk(inner_items, position, inner_scope, trail)?;
                match trail.inline.pop() {
                    Some(_) | None => {}
                }
            } else {
                let start = module.ident.span().start();
                let (child_path, child_mod_rs) =
                    self.resolve_mod(module, position, &trail.inline)?;
                let file = self.load_file(&child_path, inner_scope, child_mod_rs)?;
                trail.mods.push(ModDecl {
                    col: Col::new(start.column),
                    file,
                    line: Line::new(start.line),
                });
            }
        }
        Ok(())
    }
}

/// Where an `include!` sits and the literal it names, if any.
struct IncludeSite {
    /// Zero-based column of the macro.
    col: Col,
    /// One-based line of the macro.
    line: Line,
    /// The string literal argument, or `None` when it is not a literal.
    literal: Option<Text>,
}

/// Visitor that collects every `include!` invocation in a file.
struct IncludeFinder {
    /// The `include!` sites found so far.
    found: Vec<IncludeSite>,
}

impl<'ast> Visit<'ast> for IncludeFinder {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac
            .path
            .segments
            .last()
            .is_some_and(|last| last.ident == "include")
        {
            let start = match mac.path.segments.first() {
                Some(first) => first.ident.span().start(),
                None => proc_macro2::LineColumn { line: 1, column: 0 },
            };
            let literal = match syn::parse2::<syn::LitStr>(mac.tokens.clone()) {
                Ok(text) => Some(Text::from_string(text.value())),
                Err(_) => None,
            };
            self.found.push(IncludeSite {
                col: Col::new(start.column),
                line: Line::new(start.line),
                literal,
            });
        }
        visit::visit_macro(self, mac);
    }
}

/// Loads every file reachable from the root file of `target`.
pub(crate) fn load_crate(
    root: &Root,
    member_dir: &RelPath,
    target: &Target,
) -> Result<LoadedCrate, Failure> {
    let mut loader = Loader {
        files: Vec::new(),
        issues: Vec::new(),
        member_dir: member_dir.clone(),
        root,
    };
    let root_id = loader.load_file(&target.root, Scope::of_target(target.kind), ModRs::Yes)?;
    if root_id.index().get() != 0 {
        return Err(Failure::internal(&Message::of(
            "the crate root must be the first loaded file",
        )));
    }
    Ok(LoadedCrate {
        files: loader.files,
        issues: loader.issues,
        kind: target.kind,
    })
}

/// Builds a one-file crate from in-memory source, for canaries and the self-test.
pub(crate) fn in_memory(path: &RelPath, source: &Source) -> Result<LoadedCrate, Failure> {
    let ast = syn::parse_file(source.as_str()).map_err(|error| parse_failure(path, &error))?;
    let mut finder = IncludeFinder { found: Vec::new() };
    finder.visit_file(&ast);
    let issues = finder
        .found
        .into_iter()
        .filter(|site| site.literal.is_none())
        .map(|site| Finding {
            col: one_based(site.col),
            item: Text::from_string(String::new()),
            line: site.line,
            message: Message::of("include! with a non-literal argument cannot be scanned"),
            path: path.clone(),
            rule: Name::of(rules::UNSCANNABLE_INCLUDE.name()),
        })
        .collect();
    Ok(LoadedCrate {
        files: vec![LoadedFile {
            ast,
            includes: Vec::new(),
            mods: Vec::new(),
            path: path.clone(),
            scope: Scope::Production,
            source: source.clone(),
        }],
        issues,
        kind: TargetKind::Lib,
    })
}

/// Turns a `syn` parse error into a failure at the file and line.
fn parse_failure(path: &RelPath, error: &syn::Error) -> Failure {
    Failure::parse_at(
        path,
        Line::new(error.span().start().line),
        &Message::from_string(format!("cannot parse Rust source: {error}")),
    )
}

/// The value of a `#[path = "..."]` attribute, if one is present.
#[must_use]
fn mod_path_attr(attrs_list: &[syn::Attribute]) -> Option<Text> {
    for attr in attrs_list {
        if let Meta::NameValue(name_value) = &attr.meta {
            if name_value.path.is_ident("path") {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(text),
                    ..
                }) = &name_value.value
                {
                    return Some(Text::from_string(text.value()));
                }
            }
        }
    }
    None
}

/// Whether the attributes contain a `cfg` that selects `test`.
pub(crate) fn has_cfg_test(attrs_list: &[syn::Attribute]) -> Answer {
    attrs_list
        .iter()
        .any(|attr| match &attr.meta {
            Meta::List(list) if list.path.is_ident("cfg") => match attrs::nested(&attr.meta) {
                Some(items) => items
                    .iter()
                    .any(|item| attrs::pred_is_test(item) == Answer::Yes),
                None => false,
            },
            Meta::Path(_) | Meta::List(_) | Meta::NameValue(_) => false,
        })
        .answer()
}

/// A zero-based source column as the one-based column findings print.
fn one_based(col: Col) -> Col {
    match Col::from_zero_based(Count::new(col.get())) {
        Ok(shifted) => shifted,
        Err(_) => col,
    }
}

/// `base/rel`, normalising `.` and `..` segments.
pub(crate) fn join_relative(base: &RelPath, rel: &Text) -> RelPath {
    let mut parts: Vec<&str> = base
        .as_str()
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for part in rel.as_str().split('/') {
        match part {
            "" | "." => {}
            ".." => match parts.pop() {
                Some(_) | None => {}
            },
            other => parts.push(other),
        }
    }
    RelPath::from_string(parts.join("/"))
}

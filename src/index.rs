//! Per-module name tables and a path resolver: `use` trees (renames, groups,
//! `self`, `super`, `crate`, `extern crate`), declared items, workspace type
//! aliases and re-exports, glob imports.
use alloc::collections::{BTreeMap, BTreeSet};

use core::iter;

use syn::{Item, UseTree};

use crate::loader::{FileId, LoadedCrate, LoadedFile};

use crate::rules::PatternTable;

use crate::types::{Answer, Col, Count, Holds as _, Line, Name, Text};

use crate::util::skip_items;

/// The kind of item a module declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ItemKind {
    /// A type alias, with the path it names when that is a plain path.
    Alias(Vec<Name>),
    /// A `const` or `static`.
    Const,
    /// An enum, with the names of its variants.
    Enum(Vec<Name>),
    /// A function.
    Fn,
    /// A macro.
    Macro,
    /// A module.
    Module,
    /// A struct.
    Struct,
    /// A trait.
    Trait,
    /// A union.
    Union,
}

/// A name brought into scope by a `use` or `extern crate`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UseBinding {
    /// The name the binding introduces.
    pub name: Name,
    /// The full path the name stands for.
    pub target: Vec<Name>,
}

/// The name table of one module.
#[derive(Clone, Debug)]
pub(crate) struct ModuleData {
    /// Structs, unions and enum variants the module declares with no fields: a
    /// struct or union by name, a variant as `Enum::Variant`.
    pub fieldless: BTreeSet<Name>,
    /// Targets of glob imports in the module.
    pub globs: Vec<Vec<Name>>,
    /// Items the module declares, by name.
    pub items: BTreeMap<Name, ItemKind>,
    /// Named `use` bindings in the module.
    pub uses: Vec<UseBinding>,
}

impl Default for ModuleData {
    /// A module that declares and imports nothing yet, as `or_default` needs.
    fn default() -> Self {
        Self {
            fieldless: BTreeSet::new(),
            globs: Vec::new(),
            items: BTreeMap::new(),
            uses: Vec::new(),
        }
    }
}

/// The name tables of every module of one crate.
#[derive(Clone, Debug)]
pub(crate) struct CrateIndex {
    /// Tables for each module, keyed by module path from the crate root.
    pub modules: BTreeMap<Vec<Name>, ModuleData>,
}

impl CrateIndex {
    /// An index with no modules, to be filled as the crate is walked.
    pub(crate) const fn empty() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }
}

/// Names visible only inside a block (`use` statements and local items).
#[derive(Clone, Debug)]
pub(crate) struct BlockScope {
    /// Enums declared in the block, with the names of their variants.
    pub enums: BTreeMap<Name, Vec<Name>>,
    /// Targets of glob imports in the block.
    pub globs: Vec<Vec<Name>>,
    /// Names of items declared in the block.
    pub items: BTreeSet<Name>,
    /// Named `use` bindings in the block.
    pub uses: Vec<UseBinding>,
}

impl BlockScope {
    /// A scope that declares and imports nothing yet.
    pub(crate) const fn empty() -> Self {
        Self {
            enums: BTreeMap::new(),
            globs: Vec::new(),
            items: BTreeSet::new(),
            uses: Vec::new(),
        }
    }
}

/// Name tables for every workspace member and the external crate names it knows.
#[derive(Clone, Debug)]
pub(crate) struct WorkspaceIndex {
    /// Dependency names that differ from the library name they stand for, such as
    /// a renamed dependency key or a `package = ".."` rename, mapped to that
    /// library name.
    pub aliases: BTreeMap<Name, Name>,
    /// Names of external crates declared with `extern crate`.
    pub externs: BTreeSet<Name>,
    /// Library crates of workspace members, by library name (`[lib] name`, else
    /// the package name with `-` turned into `_`).
    pub libs: BTreeMap<Name, CrateIndex>,
}

impl WorkspaceIndex {
    /// The library name a dependency name stands for: itself unless it is an alias.
    fn canonical<'name>(&'name self, name: &'name Name) -> &'name Name {
        match self.aliases.get(name) {
            Some(library) => library,
            None => name,
        }
    }
}

/// How a path resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A single-segment name that a non-workspace glob import may provide.
    Ambiguous,
    /// A path into another crate.
    External,
    /// A name bound in a block (local `let`, local item).
    Local,
    /// Nothing in scope answers to this name.
    Unresolved,
    /// A workspace enum variant.
    Variant,
    /// A workspace item (not a variant).
    Workspace,
}

/// The result of resolving a path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Resolved {
    /// What the path resolved to.
    pub kind: Kind,
    /// The absolute path segments, with the crate name first.
    pub segments: Vec<Name>,
}

/// A path such as `std::fs::*`, where `*` matches any one segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathPattern(String);

impl PathPattern {
    /// Builds a pattern from text such as `std::fs::*`.
    pub(crate) fn of(text: &str) -> Self {
        Self(text.to_owned())
    }
}

impl Resolved {
    /// Is `pattern` at or above this path?
    pub(crate) fn is_at_or_under(&self, pattern: &PathPattern) -> Answer {
        let wanted: Vec<&str> = pattern.0.split("::").collect();
        if wanted.len() > self.segments.len() {
            return Answer::No;
        }
        wanted
            .iter()
            .zip(self.segments.iter())
            .all(|(want, have)| *want == "*" || *want == have.as_str())
            .answer()
    }

    /// Is this path at or under any entry of `table`?
    pub(crate) fn is_in_table(&self, table: PatternTable) -> Answer {
        table
            .entries()
            .iter()
            .any(|entry| self.is_at_or_under(&PathPattern::of(entry)) == Answer::Yes)
            .answer()
    }

    /// Is this path a prefix of `pattern`, so that it merely names a module on the way?
    pub(crate) fn is_prefix_of(&self, pattern: &PathPattern) -> Answer {
        let wanted: Vec<&str> = pattern.0.split("::").collect();
        (self.segments.len() <= wanted.len()
            && self
                .segments
                .iter()
                .zip(wanted.iter())
                .all(|(have, want)| *want == "*" || *want == have.as_str()))
        .answer()
    }

    /// The path as text, with segments joined by `::`.
    pub(crate) fn text(&self) -> Text {
        Text::from_string(
            self.segments
                .iter()
                .map(Name::as_str)
                .collect::<Vec<_>>()
                .join("::"),
        )
    }
}

/// Resolves names in one module of one crate.
pub(crate) struct Resolver<'ctx> {
    /// The table of the crate being resolved.
    pub own: &'ctx CrateIndex,
    /// Tables of all workspace library crates.
    pub workspace: &'ctx WorkspaceIndex,
}

/// What to resolve, and where it is written.
pub(crate) struct Query<'query> {
    /// Block scopes enclosing the path, outermost first.
    pub blocks: &'query [BlockScope],
    /// Path of the module the path is written in.
    pub module: &'query [Name],
    /// The path segments as written.
    pub segments: &'query [Name],
    /// Path of the enclosing `impl` self type, used to resolve `Self`.
    pub self_ty: Option<&'query [Name]>,
}

/// How deep resolution has recursed; bounded so cyclic imports end.
#[derive(Clone, Copy)]
struct Depth(Count);

impl Depth {
    /// A depth one level deeper, or `None` once the limit is reached.
    #[must_use]
    fn deeper(self) -> Option<Self> {
        if self.0.get() >= 24 {
            return None;
        }
        match self.0.next() {
            Ok(next) => Some(Self(next)),
            Err(_) => None,
        }
    }

    /// The starting depth, zero.
    const fn start() -> Self {
        Self(Count::new(0))
    }
}

/// A crate's tables together with how its root is spelled.
#[derive(Clone)]
struct Origin<'ctx> {
    /// The tables of the crate.
    index: &'ctx CrateIndex,
    /// How the crate root is spelled in absolute paths.
    root: Name,
}

/// A glob import and the module that contains it.
struct GlobPlace<'glob> {
    /// The glob path being imported.
    glob: &'glob [Name],
    /// Path of the module containing the glob import.
    module: &'glob [Name],
}

/// The outcome of looking a name up through a glob import.
enum GlobAnswer {
    /// The name is provided; holds its absolute path.
    Found(Vec<Name>),
    /// The glob definitely does not provide the name.
    Missing,
    /// The glob cannot be inspected, so it may provide the name.
    Opaque,
}

/// A path written in a module, with the crate it is written in.
struct Site<'site, 'ctx> {
    /// Path of the module the path is written in.
    module: &'site [Name],
    /// The crate the module belongs to.
    origin: &'site Origin<'ctx>,
}

impl<'ctx> Resolver<'ctx> {
    /// Resolves `walk` when the module declares the segment itself.
    #[must_use]
    fn declared_item(
        &self,
        site: &Site<'_, 'ctx>,
        data: &ModuleData,
        walk: &Walk<'_>,
        depth: Depth,
    ) -> Option<Resolved> {
        let (remaining, absolute) = (walk.remaining, walk.absolute);
        match data.items.get(walk.segment) {
            Some(ItemKind::Alias(target)) if !target.is_empty() => {
                let mut next = target.clone();
                next.extend(remaining.iter().cloned());
                let query = Query {
                    blocks: &[],
                    module: site.module,
                    segments: &next,
                    self_ty: None,
                };
                Some(self.resolve_in(site.origin, &query, depth))
            }
            Some(ItemKind::Enum(variants)) => {
                let is_variant = match remaining {
                    [only] => variants.contains(only),
                    _ => false,
                };
                Some(Resolved {
                    kind: if is_variant {
                        Kind::Variant
                    } else {
                        Kind::Workspace
                    },
                    segments: absolute.to_vec(),
                })
            }
            Some(
                ItemKind::Alias(_)
                | ItemKind::Module
                | ItemKind::Struct
                | ItemKind::Trait
                | ItemKind::Fn
                | ItemKind::Const
                | ItemKind::Macro
                | ItemKind::Union,
            ) => Some(Resolved {
                kind: Kind::Workspace,
                segments: absolute.to_vec(),
            }),
            None => None,
        }
    }

    /// A path whose first segment names another crate.
    fn external(&self, target: &ExternalPath<'_>) -> Resolved {
        let Some((first, rest)) = target.path.split_first() else {
            return unresolved(target.original);
        };
        let canonical = self.workspace.canonical(first);
        if let Some(lib) = self.workspace.libs.get(canonical) {
            let origin = Origin {
                index: lib,
                root: canonical.clone(),
            };
            let mut absolute = vec![canonical.clone()];
            absolute.extend(rest.iter().cloned());
            return self.follow(&origin, &absolute, Depth::start());
        }
        let mut segments = vec![normalise_crate(first)];
        segments.extend(rest.iter().cloned());
        Resolved {
            kind: Kind::External,
            segments,
        }
    }

    /// The crate paths that the glob imports in scope at `query` import from
    /// outside the workspace (`use std::*;` gives `std`), nearest scope first.
    pub(crate) fn external_globs(&self, query: &Query<'_>) -> Vec<Vec<Name>> {
        let own = Origin {
            index: self.own,
            root: Name::of("crate"),
        };
        let module_globs = self
            .own
            .modules
            .get(query.module)
            .map(|data| data.globs.as_slice())
            .into_iter();
        let block_globs = query
            .blocks
            .iter()
            .rev()
            .map(|block| block.globs.as_slice());
        let mut found: Vec<Vec<Name>> = Vec::new();
        for globs in block_globs.chain(module_globs) {
            for glob in globs {
                let target = self.resolve_in(
                    &own,
                    &Query {
                        blocks: &[],
                        module: query.module,
                        segments: glob,
                        self_ty: None,
                    },
                    Depth::start(),
                );
                if target.kind == Kind::External && !found.contains(&target.segments) {
                    found.push(target.segments);
                }
            }
        }
        found
    }

    /// Resolves a path whose first segment is either a known external crate or unresolvable.
    fn external_or_unresolved(&self, first: &Name, segments: &[Name]) -> Resolved {
        if self.is_extern(first) == Answer::Yes {
            self.external(&ExternalPath {
                original: segments,
                path: segments,
            })
        } else {
            unresolved(segments)
        }
    }

    /// Follows module walks, type aliases and re-exports to a final path.
    fn follow(&self, origin: &Origin<'ctx>, absolute: &[Name], depth: Depth) -> Resolved {
        let Some(deeper) = depth.deeper() else {
            return Resolved {
                kind: Kind::Workspace,
                segments: absolute.to_vec(),
            };
        };
        let mut module: Vec<Name> = Vec::new();
        let mut rest = skip_items(absolute, Count::new(1));
        while let Some((segment, remaining)) = rest.split_first() {
            let mut candidate = module.clone();
            candidate.push(segment.clone());
            if origin.index.modules.contains_key(&candidate) {
                module = candidate;
                rest = remaining;
                continue;
            }
            let Some(data) = origin.index.modules.get(&module) else {
                break;
            };
            let site = Site {
                module: &module,
                origin,
            };
            let walk = Walk {
                absolute,
                remaining,
                segment,
            };
            if let Some(done) = self.follow_item(&site, data, &walk, deeper) {
                return done;
            }
            break;
        }
        Resolved {
            kind: Kind::Workspace,
            segments: absolute.to_vec(),
        }
    }

    /// Follows one path segment through a module's declarations, imports and globs.
    #[must_use]
    fn follow_item(
        &self,
        site: &Site<'_, 'ctx>,
        data: &ModuleData,
        walk: &Walk<'_>,
        depth: Depth,
    ) -> Option<Resolved> {
        let (segment, remaining) = (walk.segment, walk.remaining);
        if let Some(done) = self.declared_item(site, data, walk, depth) {
            return Some(done);
        }
        if let Some(binding) = data.uses.iter().find(|binding| &binding.name == segment) {
            let mut next = binding.target.clone();
            next.extend(remaining.iter().cloned());
            let query = Query {
                blocks: &[],
                module: site.module,
                segments: &next,
                self_ty: None,
            };
            return Some(self.resolve_use_target(site.origin, &query, &next, depth));
        }
        for glob in &data.globs {
            match self.glob_lookup(
                site.origin,
                &GlobPlace {
                    glob,
                    module: site.module,
                },
                segment,
                depth,
            ) {
                GlobAnswer::Found(mut found) => {
                    found.extend(remaining.iter().cloned());
                    return Some(self.follow(site.origin, &found, depth));
                }
                GlobAnswer::Opaque | GlobAnswer::Missing => {}
            }
        }
        None
    }

    /// Asks whether the glob import `place` provides `wanted`.
    fn glob_lookup(
        &self,
        origin: &Origin<'ctx>,
        place: &GlobPlace<'_>,
        wanted: &Name,
        depth: Depth,
    ) -> GlobAnswer {
        let (module, glob) = (place.module, place.glob);
        let Some(deeper) = depth.deeper() else {
            return GlobAnswer::Opaque;
        };
        let query = Query {
            blocks: &[],
            module,
            segments: glob,
            self_ty: None,
        };
        let target = self.resolve_in(origin, &query, deeper);
        match target.kind {
            Kind::External | Kind::Ambiguous | Kind::Unresolved | Kind::Local => GlobAnswer::Opaque,
            Kind::Variant => GlobAnswer::Missing,
            Kind::Workspace => self.glob_over_workspace(origin, &target, wanted),
        }
    }

    /// Asks whether a glob over a workspace module or enum provides `wanted`.
    fn glob_over_workspace(
        &self,
        origin: &Origin<'ctx>,
        target: &Resolved,
        wanted: &Name,
    ) -> GlobAnswer {
        let Some(target_origin) = self.locate(&target.segments, origin) else {
            return GlobAnswer::Opaque;
        };
        let Some((root, inner)) = target.segments.split_first() else {
            return GlobAnswer::Missing;
        };
        let spelled = |tail: &[Name]| -> Vec<Name> {
            let mut absolute = vec![root.clone()];
            absolute.extend(tail.iter().cloned());
            absolute.push(wanted.clone());
            absolute
        };
        if let Some(data) = target_origin.index.modules.get(inner) {
            if data.items.contains_key(wanted)
                || data.uses.iter().any(|binding| &binding.name == wanted)
            {
                return GlobAnswer::Found(spelled(inner));
            }
            return if data.globs.is_empty() {
                GlobAnswer::Missing
            } else {
                GlobAnswer::Opaque
            };
        }
        // A glob over an enum.
        let Some((enum_name, enum_module)) = inner.split_last() else {
            return GlobAnswer::Missing;
        };
        let Some(data) = target_origin.index.modules.get(enum_module) else {
            return GlobAnswer::Missing;
        };
        match data.items.get(enum_name) {
            Some(ItemKind::Enum(variants)) if variants.contains(wanted) => {
                GlobAnswer::Found(spelled(inner))
            }
            Some(
                ItemKind::Enum(_)
                | ItemKind::Module
                | ItemKind::Struct
                | ItemKind::Alias(_)
                | ItemKind::Trait
                | ItemKind::Fn
                | ItemKind::Const
                | ItemKind::Macro
                | ItemKind::Union,
            )
            | None => GlobAnswer::Missing,
        }
    }

    /// Is `name` a crate that is reachable by name, such as `std` or a workspace library?
    fn is_extern(&self, name: &Name) -> Answer {
        (matches!(name.as_str(), "std" | "core" | "alloc")
            || self.workspace.libs.contains_key(name)
            || self.workspace.externs.contains(name))
        .answer()
    }

    /// Does the workspace struct or enum variant `resolved` name declare no
    /// fields? `No` when it is not known (a block item, an external path).
    pub(crate) fn is_fieldless(&self, resolved: &Resolved) -> Answer {
        let own = Origin {
            index: self.own,
            root: Name::of("crate"),
        };
        let Some(origin) = self.locate(&resolved.segments, &own) else {
            return Answer::No;
        };
        let keep: usize = match resolved.kind {
            Kind::Variant => 2,
            Kind::Workspace => 1,
            Kind::Ambiguous | Kind::External | Kind::Local | Kind::Unresolved => {
                return Answer::No;
            }
        };
        let mut rest = resolved.segments.clone();
        let mut tail: Vec<Name> = Vec::new();
        for _ in 0..keep {
            if let Some(last) = rest.pop() {
                tail.push(last);
            }
        }
        tail.reverse();
        let key = Name::from_string(tail.iter().map(Name::as_str).collect::<Vec<_>>().join("::"));
        let module: Vec<Name> = rest.into_iter().skip(1).collect();
        origin
            .index
            .modules
            .get(&module)
            .is_some_and(|data| data.fieldless.contains(&key))
            .answer()
    }

    /// Maps a resolved workspace path back to the crate it lives in.
    #[must_use]
    fn locate(&self, segments: &[Name], origin: &Origin<'ctx>) -> Option<Origin<'ctx>> {
        let (first, _) = segments.split_first()?;
        if first.as_str() == "crate" || first == &origin.root {
            return Some(origin.clone());
        }
        let canonical = self.workspace.canonical(first);
        let lib = self.workspace.libs.get(canonical)?;
        Some(Origin {
            index: lib,
            root: canonical.clone(),
        })
    }

    /// Resolves `query` as written in its module, seeing its block scopes.
    pub(crate) fn resolve(&self, query: &Query<'_>) -> Resolved {
        let own = Origin {
            index: self.own,
            root: Name::of("crate"),
        };
        self.resolve_in(&own, query, Depth::start())
    }

    /// Names bound by block-level `use` statements and local items.
    #[must_use]
    fn resolve_block_names(
        &self,
        origin: &Origin<'ctx>,
        query: &Query<'_>,
        depth: Depth,
    ) -> Option<Resolved> {
        let (first, rest) = query.segments.split_first()?;
        for block in query.blocks.iter().rev() {
            if block.items.contains(first) {
                return Some(block_item(query.blocks, query.segments));
            }
            let glob_variant = block.globs.iter().any(|glob| match glob.as_slice() {
                [only] => {
                    rest.is_empty()
                        && names_variant(block_enum(query.blocks, only), first) == Answer::Yes
                }
                _ => false,
            });
            if glob_variant {
                return Some(Resolved {
                    kind: Kind::Variant,
                    segments: query.segments.to_vec(),
                });
            }
            if let Some(binding) = block
                .uses
                .iter()
                .rev()
                .find(|binding| &binding.name == first)
            {
                let mut target = binding.target.clone();
                target.extend(rest.iter().cloned());
                if matches!(target.first(), Some(head) if block_enum(query.blocks, head).is_some())
                {
                    return Some(block_item(query.blocks, &target));
                }
                let inner = Query {
                    blocks: &[],
                    module: query.module,
                    segments: &target,
                    self_ty: query.self_ty,
                };
                return Some(self.resolve_in(origin, &inner, depth));
            }
        }
        None
    }

    /// Resolves the first segment through glob imports, then external crates.
    fn resolve_globs(
        &self,
        origin: &Origin<'ctx>,
        query: &Query<'_>,
        data: &ModuleData,
        depth: Depth,
    ) -> Resolved {
        let Some((first, rest)) = query.segments.split_first() else {
            return unresolved(query.segments);
        };
        let mut ambiguous = false;
        let glob_lists = query
            .blocks
            .iter()
            .rev()
            .map(|block| &block.globs)
            .chain(iter::once(&data.globs));
        for globs in glob_lists {
            for glob in globs {
                match self.glob_lookup(
                    origin,
                    &GlobPlace {
                        glob,
                        module: query.module,
                    },
                    first,
                    depth,
                ) {
                    GlobAnswer::Found(mut absolute) => {
                        absolute.extend(rest.iter().cloned());
                        return self.follow(origin, &absolute, depth);
                    }
                    GlobAnswer::Opaque => ambiguous = true,
                    GlobAnswer::Missing => {}
                }
            }
        }
        if self.is_extern(first) == Answer::Yes {
            return self.external(&ExternalPath {
                original: query.segments,
                path: query.segments,
            });
        }
        if ambiguous {
            return Resolved {
                kind: Kind::Ambiguous,
                segments: query.segments.to_vec(),
            };
        }
        unresolved(query.segments)
    }

    /// Resolves `query` within the crate `origin`, handling keywords and scopes.
    fn resolve_in(&self, origin: &Origin<'ctx>, query: &Query<'_>, depth: Depth) -> Resolved {
        let Some((first, rest)) = query.segments.split_first() else {
            return unresolved(query.segments);
        };
        let Some(deeper) = depth.deeper() else {
            return unresolved(query.segments);
        };
        match first.as_str() {
            "::" => {
                return self.external(&ExternalPath {
                    original: query.segments,
                    path: rest,
                })
            }
            "crate" => {
                let mut absolute = vec![origin.root.clone()];
                absolute.extend(rest.iter().cloned());
                return self.follow(origin, &absolute, deeper);
            }
            "self" => {
                let mut absolute = vec![origin.root.clone()];
                absolute.extend(query.module.iter().cloned());
                absolute.extend(rest.iter().cloned());
                return self.follow(origin, &absolute, deeper);
            }
            "super" => return self.resolve_super(origin, query, rest, deeper),
            "Self" => {
                return match query.self_ty {
                    Some(own_type) => {
                        let mut absolute = own_type.to_vec();
                        absolute.extend(rest.iter().cloned());
                        self.follow(origin, &absolute, deeper)
                    }
                    None => unresolved(query.segments),
                };
            }
            _ => {}
        }
        if let Some(found) = self.resolve_block_names(origin, query, deeper) {
            return found;
        }
        let Some(data) = origin.index.modules.get(query.module) else {
            return self.external_or_unresolved(first, query.segments);
        };
        if data.items.contains_key(first) {
            let mut absolute = vec![origin.root.clone()];
            absolute.extend(query.module.iter().cloned());
            absolute.push(first.clone());
            absolute.extend(rest.iter().cloned());
            return self.follow(origin, &absolute, deeper);
        }
        if let Some(binding) = data.uses.iter().find(|binding| &binding.name == first) {
            let mut target = binding.target.clone();
            target.extend(rest.iter().cloned());
            return self.resolve_use_target(origin, query, &target, deeper);
        }
        self.resolve_globs(origin, query, data, deeper)
    }

    /// Resolves a path that starts with `super`.
    fn resolve_super(
        &self,
        origin: &Origin<'ctx>,
        query: &Query<'_>,
        rest: &[Name],
        depth: Depth,
    ) -> Resolved {
        let mut parent: Vec<Name> = query.module.to_vec();
        match parent.pop() {
            Some(_) | None => {}
        }
        let mut remaining = rest;
        while let Some((head, after)) = remaining.split_first() {
            if head.as_str() != "super" {
                break;
            }
            match parent.pop() {
                Some(_) | None => {}
            }
            remaining = after;
        }
        let mut absolute = vec![origin.root.clone()];
        absolute.extend(parent);
        absolute.extend(remaining.iter().cloned());
        self.follow(origin, &absolute, depth)
    }

    /// Resolves the target of a `use` binding.
    fn resolve_use_target(
        &self,
        origin: &Origin<'ctx>,
        query: &Query<'_>,
        target: &[Name],
        depth: Depth,
    ) -> Resolved {
        if target.first().map(Name::as_str) == Some("::") {
            let tail = skip_items(target, Count::new(1));
            return self.external(&ExternalPath {
                original: target,
                path: tail,
            });
        }
        let inner = Query {
            blocks: &[],
            module: query.module,
            segments: target,
            self_ty: query.self_ty,
        };
        self.resolve_in(origin, &inner, depth)
    }
}

/// One step of following a path through a module.
struct Walk<'walk> {
    /// The full absolute path being followed.
    absolute: &'walk [Name],
    /// The segments after the current one.
    remaining: &'walk [Name],
    /// The segment being looked up.
    segment: &'walk Name,
}

/// A path to look up in another crate, with the spelling to report on failure.
struct ExternalPath<'pat> {
    /// The path as written, reported if resolution fails.
    original: &'pat [Name],
    /// The path to resolve, without a leading `::`.
    path: &'pat [Name],
}

/// The variants of the enum `name` declared in the innermost block that declares it.
#[must_use]
fn block_enum<'blocks>(blocks: &'blocks [BlockScope], name: &Name) -> Option<&'blocks [Name]> {
    blocks
        .iter()
        .rev()
        .find(|block| block.items.contains(name))
        .and_then(|block| block.enums.get(name))
        .map(Vec::as_slice)
}

/// Is `variant` one of the `variants` of a block-declared enum?
fn names_variant(variants: Option<&[Name]>, variant: &Name) -> Answer {
    variants
        .is_some_and(|found| found.contains(variant))
        .answer()
}

/// A path whose first segment names an item declared in a block: a variant when
/// that item is an enum and the second segment names one of its variants.
fn block_item(blocks: &[BlockScope], segments: &[Name]) -> Resolved {
    let is_variant = match segments {
        [first, second] => names_variant(block_enum(blocks, first), second),
        _ => Answer::No,
    };
    Resolved {
        kind: if is_variant == Answer::Yes {
            Kind::Variant
        } else {
            Kind::Local
        },
        segments: segments.to_vec(),
    }
}

/// `std`, `core` and `alloc` are the same crate for every rule.
pub(crate) fn normalise_crate(name: &Name) -> Name {
    match name.as_str() {
        "core" | "alloc" => Name::of("std"),
        _ => name.clone(),
    }
}

/// The identifiers of a `syn` path as names.
pub(crate) fn raw_segments(path: &syn::Path) -> Vec<Name> {
    path.segments
        .iter()
        .map(|segment| Name::from_string(segment.ident.to_string()))
        .collect()
}

/// The Rust crate identifier of a package, with `-` turned into `_`.
pub(crate) fn crate_ident(package: &Name) -> Name {
    Name::from_string(package.as_str().replace('-', "_"))
}

/// Builds the module tables of one target crate.
pub(crate) fn build_crate(loaded: &LoadedCrate) -> CrateIndex {
    let mut index = CrateIndex::empty();
    if let Some(root) = loaded.files.first() {
        index_items(&loaded.files, root, &[], &mut index);
    }
    index
}

/// Indexes the items of `file` as module `module`.
fn index_items(files: &[LoadedFile], file: &LoadedFile, module: &[Name], index: &mut CrateIndex) {
    ensure_module(index, module);
    index_item_list(files, file, &file.ast.items, module, index);
}

/// Indexes a list of items belonging to `module`.
fn index_item_list(
    files: &[LoadedFile],
    file: &LoadedFile,
    items: &[Item],
    module: &[Name],
    index: &mut CrateIndex,
) {
    for item in items {
        match item {
            Item::Mod(inner) => index_mod(files, file, inner, module, index),
            Item::Use(item_use) => {
                let mut bindings = Vec::new();
                let mut globs = Vec::new();
                flatten_use(&item_use.tree, &mut Vec::new(), &mut bindings, &mut globs);
                let data = index.modules.entry(module.to_vec()).or_default();
                data.uses.extend(bindings);
                data.globs.extend(globs);
            }
            Item::ExternCrate(extern_crate) => index_extern_crate(extern_crate, module, index),
            Item::Enum(_)
            | Item::Struct(_)
            | Item::Union(_)
            | Item::Type(_)
            | Item::Trait(_)
            | Item::Fn(_)
            | Item::Const(_)
            | Item::Static(_)
            | Item::Macro(_) => index_declaration(item, module, index),
            Item::TraitAlias(_) | Item::ForeignMod(_) | Item::Impl(_) | Item::Verbatim(_) | _ => {}
        }
    }
}

/// Indexes a `mod` item and, recursively, its contents.
fn index_mod(
    files: &[LoadedFile],
    file: &LoadedFile,
    inner: &syn::ItemMod,
    module: &[Name],
    index: &mut CrateIndex,
) {
    let name = Name::from_string(inner.ident.to_string());
    let mut path = module.to_vec();
    path.push(name.clone());
    declare(index, module, name, ItemKind::Module);
    ensure_module(index, &path);
    if let Some((_, inner_items)) = &inner.content {
        index_item_list(files, file, inner_items, &path, index);
    } else {
        let start = inner.ident.span().start();
        let key = (Line::new(start.line), Col::new(start.column));
        let child = file
            .mods
            .iter()
            .find(|decl| (decl.line, decl.col) == key)
            .map(|decl| decl.file);
        if let Some(child_file) = child.and_then(|id| files.get(FileId::index(id).get())) {
            index_item_list(files, child_file, &child_file.ast.items, &path, index);
        }
    }
}

/// Indexes an `extern crate` item as a binding.
fn index_extern_crate(
    extern_crate: &syn::ItemExternCrate,
    module: &[Name],
    index: &mut CrateIndex,
) {
    let name = Name::from_string(extern_crate.ident.to_string());
    let binding = match &extern_crate.rename {
        Some((_, alias)) => Name::from_string(alias.to_string()),
        None => name.clone(),
    };
    if binding.as_str() != "_" {
        index
            .modules
            .entry(module.to_vec())
            .or_default()
            .uses
            .push(UseBinding {
                name: binding,
                target: vec![Name::of("::"), name],
            });
    }
}

/// The name and kind of the declaration a type, fn, const or macro item introduces.
#[must_use]
fn declared(item: &Item) -> Option<(Name, ItemKind)> {
    let (ident, kind) = match item {
        Item::Enum(item_enum) => {
            let variants = item_enum
                .variants
                .iter()
                .map(|variant| Name::from_string(variant.ident.to_string()))
                .collect();
            (&item_enum.ident, ItemKind::Enum(variants))
        }
        Item::Struct(item_struct) => (&item_struct.ident, ItemKind::Struct),
        Item::Union(item_union) => (&item_union.ident, ItemKind::Union),
        Item::Type(item_type) => {
            let target = match item_type.ty.as_ref() {
                syn::Type::Path(type_path) if type_path.qself.is_none() => {
                    raw_segments(&type_path.path)
                }
                _ => Vec::new(),
            };
            (&item_type.ident, ItemKind::Alias(target))
        }
        Item::Trait(item_trait) => (&item_trait.ident, ItemKind::Trait),
        Item::Fn(item_fn) => (&item_fn.sig.ident, ItemKind::Fn),
        Item::Const(item_const) => (&item_const.ident, ItemKind::Const),
        Item::Static(item_static) => (&item_static.ident, ItemKind::Const),
        Item::Macro(item_macro) => (item_macro.ident.as_ref()?, ItemKind::Macro),
        Item::Mod(_)
        | Item::Use(_)
        | Item::ExternCrate(_)
        | Item::TraitAlias(_)
        | Item::ForeignMod(_)
        | Item::Impl(_)
        | Item::Verbatim(_)
        | _ => return None,
    };
    Some((Name::from_string(ident.to_string()), kind))
}

/// Declares the name introduced by a type, fn, const or macro item.
fn index_declaration(item: &Item, module: &[Name], index: &mut CrateIndex) {
    let mut empty: Vec<Name> = Vec::new();
    match item {
        Item::Struct(item_struct) if no_fields(&item_struct.fields) == Answer::Yes => {
            empty.push(Name::from_string(item_struct.ident.to_string()));
        }
        Item::Enum(item_enum) => {
            for variant in &item_enum.variants {
                if no_fields(&variant.fields) == Answer::Yes {
                    empty.push(Name::from_string(format!(
                        "{}::{}",
                        item_enum.ident, variant.ident
                    )));
                }
            }
        }
        _ => {}
    }
    if !empty.is_empty() {
        index
            .modules
            .entry(module.to_vec())
            .or_default()
            .fieldless
            .extend(empty);
    }
    if let Some((name, kind)) = declared(item) {
        declare(index, module, name, kind);
    }
}

/// Does a struct or variant declare no fields?
fn no_fields(fields: &syn::Fields) -> Answer {
    match fields {
        syn::Fields::Unit => Answer::Yes,
        syn::Fields::Named(named) => named.named.is_empty().answer(),
        syn::Fields::Unnamed(unnamed) => unnamed.unnamed.is_empty().answer(),
    }
}

/// Records that `module` declares `name` as a `kind` item.
fn declare(index: &mut CrateIndex, module: &[Name], name: Name, kind: ItemKind) {
    index
        .modules
        .entry(module.to_vec())
        .or_default()
        .items
        .extend([(name, kind)]);
}

/// Makes sure `module` has a table, even when it declares nothing.
fn ensure_module(index: &mut CrateIndex, module: &[Name]) {
    if !index.modules.contains_key(module) {
        index
            .modules
            .extend([(module.to_vec(), ModuleData::default())]);
    }
}

/// Flattens a `use` tree into named bindings and glob targets.
pub(crate) fn flatten_use(
    tree: &UseTree,
    prefix: &mut Vec<Name>,
    bindings: &mut Vec<UseBinding>,
    globs: &mut Vec<Vec<Name>>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(Name::from_string(path.ident.to_string()));
            flatten_use(&path.tree, prefix, bindings, globs);
            match prefix.pop() {
                Some(_) | None => {}
            }
        }
        UseTree::Name(name) => {
            let ident = name.ident.to_string();
            if ident == "self" {
                if let Some(last) = prefix.last() {
                    bindings.push(UseBinding {
                        name: last.clone(),
                        target: prefix.clone(),
                    });
                }
            } else {
                let mut target = prefix.clone();
                target.push(Name::from_string(ident.clone()));
                bindings.push(UseBinding {
                    name: Name::from_string(ident),
                    target,
                });
            }
        }
        UseTree::Rename(rename) => {
            let ident = rename.ident.to_string();
            let mut target = prefix.clone();
            if ident != "self" {
                target.push(Name::from_string(ident));
            }
            let alias = rename.rename.to_string();
            if alias != "_" {
                bindings.push(UseBinding {
                    name: Name::from_string(alias),
                    target,
                });
            }
        }
        UseTree::Glob(_) => globs.push(prefix.clone()),
        UseTree::Group(group) => {
            for inner in &group.items {
                flatten_use(inner, prefix, bindings, globs);
            }
        }
    }
}

/// Builds an unresolved result for `segments`.
fn unresolved(segments: &[Name]) -> Resolved {
    let mut shown: Vec<Name> = segments.to_vec();
    if let Some(first) = shown.first_mut() {
        *first = normalise_crate(first);
    }
    Resolved {
        kind: Kind::Unresolved,
        segments: shown,
    }
}

//! Scans a whole workspace: discovery, loading, indexing, every rule.
use alloc::collections::{BTreeMap, BTreeSet};

use syn::Item;

use crate::config::{Config, MemberConfig};

use crate::failure::Failure;

use crate::findings::{self, Finding};

use crate::index::{self, CrateIndex, Resolver, WorkspaceIndex};

use crate::io::{DirEntry, EntryKind, RealPath, Root};

use crate::loader::{self, LoadedCrate};

use crate::rules::{self, PatternTable};

use crate::scan::{self, Env, ExpectUse};

use crate::toml::{Doc, SectionKind};

use crate::types::{Answer, Col, Holds as _, Line, Message, Name, RelPath, Text};

use crate::util::is_rust_file;

use crate::workspace::{self, Member, TargetKind, Workspace};

/// The outcome of scanning a workspace.
pub(crate) struct Analysis {
    /// Every `expect` attribute use found, sorted and deduplicated.
    pub expects: Vec<ExpectUse>,
    /// The normalised findings of every rule.
    pub findings: Vec<Finding>,
}

/// One loaded cargo target with its member and index.
struct Loaded {
    /// The role configuration of the owning member.
    config: MemberConfig,
    /// The item index of this target's crate.
    index: CrateIndex,
    /// The parsed source files of the target.
    krate: LoadedCrate,
    /// The workspace member that owns the target.
    member: Member,
}

/// Everything loading the workspace's targets found.
struct Discovered {
    /// Crate names a path may start with: dependencies and workspace members.
    externs: BTreeSet<Name>,
    /// Every loaded cargo target.
    loaded: Vec<Loaded>,
    /// Source files that some target reaches through `mod` or includes.
    reached: BTreeSet<RelPath>,
}

/// What `unreached_files` needs to judge each directory entry.
struct OrphanScan<'scan> {
    /// The configuration, for its canary directories.
    config: &'scan Config,
    /// Directories of all workspace members, so nested members are skipped.
    member_dirs: Vec<&'scan RelPath>,
    /// Files already reached by a target.
    reached: &'scan BTreeSet<RelPath>,
    /// The reached files with symbolic links resolved: a file reached through a
    /// link is reached under its real name too.
    reached_real: BTreeSet<RealPath>,
    /// The workspace root used to list directories.
    root: &'scan Root,
}

/// The state of one member's orphan walk.
struct Walk {
    /// The orphan and link findings so far.
    found: Vec<Finding>,
    /// Directories already walked, by real path, so a link cycle ends.
    seen: BTreeSet<RealPath>,
}

impl OrphanScan<'_> {
    /// Does the walk go into `path`? Hidden directories too, except the
    /// version-control directories and `target`, nested members and canary dirs.
    fn descends(&self, member: &Member, dir: &Name, path: &RelPath) -> Answer {
        let skip = matches!(dir.as_str(), "target" | ".git" | ".jj")
            || (*path != member.dir && self.member_dirs.contains(&path))
            || in_canary_dir(self.config, path) == Answer::Yes;
        (!skip).answer()
    }

    /// Is `path` a `.rs` file that no target reaches?
    fn is_orphan(&self, name: &Name, path: &RelPath) -> Answer {
        let reached_real = self
            .root
            .real(path)
            .is_some_and(|real| self.reached_real.contains(&real));
        (is_rust_file(name) == Answer::Yes
            && !self.reached.contains(path)
            && !reached_real
            && in_canary_dir(self.config, path) == Answer::No)
            .answer()
    }

    /// Handles a symbolic link: a link out of the workspace, or to nothing, is a
    /// finding; a link to a file or directory inside is followed like the target.
    #[must_use]
    fn link(
        &self,
        member: &Member,
        entry: &DirEntry,
        path: &RelPath,
        walk: &mut Walk,
    ) -> Option<RelPath> {
        let is_dir = self.root.is_dir(path) == Answer::Yes;
        let is_source = is_rust_file(&entry.name) == Answer::Yes;
        if self.root.exists(path) == Answer::No {
            if is_source {
                walk.found.push(link_finding(
                    path.clone(),
                    &Message::of("this symbolic link points at nothing, so cargo cannot build it"),
                ));
            }
            return None;
        }
        if !is_dir && !is_source {
            return None;
        }
        if self.root.escapes(path) == Answer::Yes {
            walk.found.push(link_finding(
                path.clone(),
                &Message::of("this symbolic link points outside the workspace, so its target is not scanned as workspace code; copy the file or the directory in"),
            ));
            return None;
        }
        if is_dir {
            return (self.descends(member, &entry.name, path) == Answer::Yes).then(|| path.clone());
        }
        if self.is_orphan(&entry.name, path) == Answer::Yes {
            walk.found.push(orphan_finding(path.clone()));
        }
        None
    }

    /// The orphan files under one member directory.
    fn member(&self, member: &Member) -> Result<Vec<Finding>, Failure> {
        let mut walk = Walk {
            found: Vec::new(),
            seen: BTreeSet::new(),
        };
        let mut pending = vec![member.dir.clone()];
        // Linked directories wait until every real directory is walked, so a file
        // is named by its real path and not by a link to it.
        let mut linked: Vec<RelPath> = Vec::new();
        loop {
            let next = match pending.pop() {
                Some(found) => Some(found),
                None => linked.pop(),
            };
            let Some(dir) = next else {
                break;
            };
            if let Some(real) = self.root.real(&dir) {
                if !walk.seen.insert(real) {
                    continue;
                }
            }
            for entry in self.root.list(&dir)? {
                let path = dir.join(entry.name.as_str());
                match entry.kind {
                    EntryKind::Dir if self.descends(member, &entry.name, &path) == Answer::Yes => {
                        pending.push(path);
                    }
                    EntryKind::File if self.is_orphan(&entry.name, &path) == Answer::Yes => {
                        walk.found.push(orphan_finding(path));
                    }
                    EntryKind::Other => {
                        linked.extend(self.link(member, &entry, &path, &mut walk));
                    }
                    EntryKind::Dir | EntryKind::File => {}
                }
            }
        }
        Ok(walk.found)
    }
}

/// Is `path` inside one of the canary directories (`canary-dirs` and the
/// `clippy-canaries` directory)?
///
/// Canary files violate rules on purpose, so scanning them as ordinary code
/// would make every canary a finding. The normal scan and `unreached-file` skip
/// them; `canaries` checks them instead, and each mark must fire on its line
/// (a diagnostic that is not marked with its own id on its own line is a
/// finding, so a canary cannot hide a second violation). The exclusion is from
/// suppression, the ledger and nothing else: canary files are judge paths,
/// append-only for additions, and modifying or deleting one needs approval
/// (`against::canary_dir_globs`).
pub(crate) fn in_canary_dir(config: &Config, path: &RelPath) -> Answer {
    let mut dirs: Vec<&str> = config.canary_dirs.iter().map(RelPath::as_str).collect();
    if let Some(clippy) = &config.clippy_canaries {
        dirs.push(clippy.dir.as_str());
    }
    dirs.iter()
        .any(|candidate| {
            let dir = candidate.trim_end_matches('/');
            path.as_str() == dir || path.as_str().starts_with(&format!("{dir}/"))
        })
        .answer()
}

/// Adds the `snake_case` names of all dependencies declared in `doc` to `out`.
fn dependency_names(doc: &Doc, out: &mut BTreeSet<Name>) {
    for section in &doc.sections {
        let Some(last) = section.header.last() else {
            continue;
        };
        let kind = last.as_str();
        if section.kind == SectionKind::Table
            && matches!(
                kind,
                "dependencies" | "dev-dependencies" | "build-dependencies"
            )
        {
            for entry in &section.entries {
                out.extend([Name::from_string(
                    entry.key.first.as_str().replace('-', "_"),
                )]);
            }
        }
        if let Some(parent) = section
            .header
            .len()
            .checked_sub(2)
            .and_then(|index| section.header.get(index))
        {
            if matches!(
                parent.as_str(),
                "dependencies" | "dev-dependencies" | "build-dependencies"
            ) {
                out.extend([Name::from_string(last.as_str().replace('-', "_"))]);
            }
        }
    }
}

/// Adds the names of single-field tuple structs in `items`, descending into inline modules.
pub(crate) fn collect_newtypes(items: &[Item], out: &mut BTreeSet<Name>) {
    for item in items {
        match item {
            Item::Struct(item_struct) => {
                if let syn::Fields::Unnamed(fields) = &item_struct.fields {
                    if fields.unnamed.len() == 1 {
                        out.extend([Name::from_string(item_struct.ident.to_string())]);
                    }
                }
            }
            Item::Mod(module) => {
                if let Some((_, inner)) = &module.content {
                    collect_newtypes(inner, out);
                }
            }
            _ => {}
        }
    }
}

/// The last path segments of every built-in and configured banned path.
pub(crate) fn banned_leaves(config: &Config) -> BTreeSet<Name> {
    let mut leaves = BTreeSet::new();
    let tables: [PatternTable; 5] = [
        rules::UNORDERED_PATHS,
        rules::AMBIENT_PATHS,
        rules::PANIC_PATHS,
        rules::MODULAR_TYPE_PATHS,
        rules::LEAK_PATHS,
    ];
    for table in tables {
        for entry in table.entries() {
            if let Some(leaf) = entry.rsplit("::").next() {
                leaves.extend([Name::of(leaf)]);
            }
        }
    }
    for ban in &config.bans {
        for entry in &ban.paths {
            if let Some(leaf) = entry.as_str().rsplit("::").next() {
                if leaf != "*" {
                    leaves.extend([Name::of(leaf)]);
                }
            }
        }
    }
    leaves
}

/// Loads every cargo target of every member, noting the files they reach and
/// the crate names a path may start with.
fn discover(root: &Root, config: &Config, ws: &Workspace) -> Result<Discovered, Failure> {
    let mut found = Discovered {
        externs: BTreeSet::new(),
        loaded: Vec::new(),
        reached: BTreeSet::new(),
    };
    dependency_names(&ws.root_manifest, &mut found.externs);
    for member in &ws.members {
        dependency_names(&member.manifest, &mut found.externs);
        found
            .externs
            .extend([index::crate_ident(&member.name), member.lib_name()]);
        let member_config = config
            .members
            .iter()
            .find(|entry| entry.name == member.name)
            .cloned()
            .ok_or_else(|| Failure::usage(&Message::of("member without a role")))?;
        for target in workspace::targets(root, member)? {
            if in_canary_dir(config, &target.root) == Answer::Yes {
                continue;
            }
            let krate = loader::load_crate(root, &member.dir, &target)?;
            for file in &krate.files {
                found.reached.extend([file.path.clone()]);
                for included in &file.includes {
                    found.reached.extend([included.path.clone()]);
                }
            }
            let crate_index = index::build_crate(&krate);
            found.loaded.push(Loaded {
                config: member_config.clone(),
                index: crate_index,
                krate,
                member: member.clone(),
            });
        }
    }
    Ok(found)
}

/// Scans every member of the workspace with every rule and returns the findings.
pub(crate) fn analyze(root: &Root, config: &Config, ws: &Workspace) -> Result<Analysis, Failure> {
    ws.check_roles(config)?;
    let Discovered {
        loaded,
        reached,
        externs,
    } = discover(root, config, ws)?;
    let mut libs: BTreeMap<Name, CrateIndex> = BTreeMap::new();
    for entry in &loaded {
        if entry.krate.kind == TargetKind::Lib {
            libs.extend([(entry.member.lib_name(), entry.index.clone())]);
        }
    }
    let workspace_index = WorkspaceIndex {
        aliases: workspace::dependency_aliases(ws),
        externs,
        libs,
    };
    let mut newtypes = BTreeSet::new();
    for entry in &loaded {
        for file in &entry.krate.files {
            collect_newtypes(&file.ast.items, &mut newtypes);
        }
    }
    let leaves = banned_leaves(config);
    let mut all_findings: Vec<Finding> = Vec::new();
    let mut all_expects: Vec<ExpectUse> = Vec::new();
    for entry in &loaded {
        let env = Env {
            banned_leaves: &leaves,
            config,
            member: &entry.config,
            newtypes: &newtypes,
            resolver: Resolver {
                own: &entry.index,
                workspace: &workspace_index,
            },
        };
        let mut output = scan::scan_crate(&env, &entry.krate);
        all_findings.append(&mut output.findings);
        all_expects.append(&mut output.expects);
    }
    all_findings.extend(unreached_files(root, config, ws, &reached)?);
    all_expects.sort();
    all_expects.dedup();
    Ok(Analysis {
        expects: all_expects,
        findings: findings::normalise(all_findings),
    })
}

/// The finding for a `.rs` file that no cargo target reaches.
fn orphan_finding(path: RelPath) -> Finding {
    Finding {
        col: Col::new(1),
        item: Text::from_string(String::new()),
        line: Line::new(1),
        message: Message::of(
            "no cargo target reaches this file; declare it with `mod` or remove it",
        ),
        path,
        rule: Name::of(rules::UNREACHED_FILE.name()),
    }
}

/// The finding for a symbolic link the scan cannot follow.
fn link_finding(path: RelPath, why: &Message) -> Finding {
    Finding {
        col: Col::new(1),
        item: Text::from_string(String::new()),
        line: Line::new(1),
        message: why.clone(),
        path,
        rule: Name::of(rules::UNREACHED_FILE.name()),
    }
}

/// Every `.rs` file under a member directory that no target reaches.
fn unreached_files(
    root: &Root,
    config: &Config,
    ws: &Workspace,
    reached: &BTreeSet<RelPath>,
) -> Result<Vec<Finding>, Failure> {
    let scan = OrphanScan {
        config,
        member_dirs: ws.members.iter().map(|member| &member.dir).collect(),
        reached,
        reached_real: reached.iter().filter_map(|path| root.real(path)).collect(),
        root,
    };
    let mut found = Vec::new();
    for member in &ws.members {
        found.extend(scan.member(member)?);
    }
    Ok(found)
}

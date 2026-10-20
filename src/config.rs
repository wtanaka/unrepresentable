//! `.unrepresentable/config`: facts the tool cannot discover, plus recorded
//! exceptions. Everything else is fixed by convention.
use crate::failure::Failure;

use crate::glob::{Brackets, Glob};

use crate::toml::{self, Doc, Entry, Section, SectionKind, Value};

use crate::types::{Answer, Count, Line, Message, Name, RelPath, Source, Text};

/// Path of the config file, relative to the repository root. It is fixed: no
/// flag or key moves it.
pub(crate) const CONFIG_FILE: &str = ".unrepresentable/config";

/// Path of the ledger file, relative to the repository root.
pub(crate) const LEDGER_FILE: &str = ".unrepresentable/ledger.md";

/// The longest crate name `must-never-reach` accepts.
const MAX_CRATE_NAME: usize = 64;

/// Keys accepted at the top level of the config file.
const TOP_KEYS: &[&str] = &[
    "baseline",
    "default-range",
    "canary-dirs",
    "clippy-canaries",
    "arith-files",
    "test-attributes",
    "silent-fallback-names",
    "fn-name-suffixes",
    "assertion-macros",
    "judge",
    "code",
    "append-only",
    "cargo-exceptions",
];

/// Whether a workspace member is pure logic or an effectful shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    /// The member is pure and may not use ambient effects unless allowed.
    Pure,
    /// The member is an effectful shell around the pure core.
    Shell,
}

/// How recorded baseline entries are treated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BaselineMode {
    /// Baseline entries are not allowed at all.
    Forbidden,
    /// Baseline entries may be removed but never added.
    ShrinkOnly,
}

/// Whether a config must state which paths are code and give every
/// `[[forbidden-dependency]]` a crate to keep out (`must-not-reach` or
/// `must-never-reach`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Strictness {
    /// A config that is read to compare against: an old one may lack `code` or
    /// have an empty `must-not-reach`.
    Lenient,
    /// The config of the run: `code` must be a non-empty list of globs.
    Required,
}

/// An ambient-effect path that a member may use in the listed files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AmbientAllow {
    /// Globs of files in which the path is allowed.
    pub files: Vec<Glob>,
    /// The ambient path (for example a std path) being allowed.
    pub path: Text,
}

/// A newtype and the methods on it that are explicitly allowed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NewtypeAllow {
    /// Names of the allowed methods.
    pub methods: Vec<Name>,
    /// Name of the newtype the allowance applies to.
    pub type_name: Name,
}

/// Per-member configuration: role and recorded exceptions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MemberConfig {
    /// Ambient-effect allowances for this member.
    pub ambient: Vec<AmbientAllow>,
    /// Name of the workspace member.
    pub name: Name,
    /// Newtype method allowances for this member.
    pub newtypes: Vec<NewtypeAllow>,
    /// Whether the member is pure or a shell.
    pub role: Role,
}

/// A named set of bans on derives, macros, methods, paths and impls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BanSet {
    /// Derive macro names that are banned.
    pub derives: Vec<Name>,
    /// Globs of files the ban applies to; all files when none are listed.
    pub files: Vec<Glob>,
    /// Identifier of the ban set, used in findings.
    pub id: Name,
    /// Trait names whose `impl` blocks are banned.
    pub impl_traits: Vec<Text>,
    /// Macro names that are banned.
    pub macros: Vec<Text>,
    /// Method names that are banned.
    pub methods: Vec<Name>,
    /// Item paths that are banned.
    pub paths: Vec<Text>,
}

/// Requires a comment with a given prefix near each use of a macro.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MacroComment {
    /// Name of the macro that needs a comment.
    pub macro_name: Text,
    /// Prefix the required comment must start with.
    pub prefix: Text,
    /// How many lines away the comment may be from the macro call.
    pub within_lines: Count,
}

/// A crate and the crates it must not depend on, directly or transitively.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ForbiddenDependency {
    /// Name of the crate the restriction starts from.
    pub crate_name: Name,
    /// Like `must_not_reach`, but a name may be absent from the dependency
    /// graph: the rule fires once the crate appears.
    pub must_never_reach: Vec<Name>,
    /// Names of crates that must not be reachable from `crate_name`; each must
    /// be a crate of the dependency graph.
    pub must_not_reach: Vec<Name>,
}

/// A restriction on which names the listed files may mention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ForbiddenReference {
    /// Globs of files the restriction applies to.
    pub files: Vec<Glob>,
    /// Texts that must not appear as names in those files.
    pub must_not_name: Vec<Text>,
}

/// Location and package name of the clippy canaries crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClippyCanaries {
    /// Directory holding the clippy canaries package.
    pub dir: RelPath,
    /// Cargo package name of the clippy canaries crate.
    pub package: Name,
}

/// A recorded exception to a rule, pinned to a file and item.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BaselineEntry {
    /// File the exception applies to.
    pub file: RelPath,
    /// Which occurrence of the item within the file is covered.
    pub index: Count,
    /// Enclosing item path of the excepted finding.
    pub item: Text,
    /// Line of the entry in the config file.
    pub line: Line,
    /// Name of the rule being excepted.
    pub rule: Name,
}

/// The parsed contents of the config file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Config {
    /// Globs of files that may only be appended to.
    pub append_only: Vec<Glob>,
    /// Globs of files checked for arithmetic rules.
    pub arith_files: Vec<Glob>,
    /// Names of macros that count as assertions.
    pub assertion_macros: Vec<Name>,
    /// Configured ban sets.
    pub bans: Vec<BanSet>,
    /// Recorded baseline exceptions.
    pub baseline: Vec<BaselineEntry>,
    /// Whether the baseline may only shrink or is forbidden.
    pub baseline_mode: BaselineMode,
    /// Directories that contain canary files.
    pub canary_dirs: Vec<RelPath>,
    /// Exact cargo config entries recorded as tracked exceptions.
    pub cargo_exceptions: Vec<Text>,
    /// The clippy canaries crate, if configured.
    pub clippy_canaries: Option<ClippyCanaries>,
    /// Globs of files that count as code.
    pub code: Vec<Glob>,
    /// Default revision range to compare against, if set.
    pub default_range: Option<Text>,
    /// Function name suffixes with special meaning to rules.
    pub fn_name_suffixes: Vec<Name>,
    /// Forbidden crate dependency restrictions.
    pub forbidden_dependencies: Vec<ForbiddenDependency>,
    /// Forbidden name reference restrictions.
    pub forbidden_references: Vec<ForbiddenReference>,
    /// Globs of files that are subject to judging.
    pub judge: Vec<Glob>,
    /// Required macro comment rules.
    pub macro_comments: Vec<MacroComment>,
    /// Per-member configuration.
    pub members: Vec<MemberConfig>,
    /// Function names that indicate a silent fallback.
    pub silent_fallback_names: Vec<Name>,
    /// Attribute texts that mark a function as a test.
    pub test_attributes: Vec<Text>,
}

impl Config {
    /// A config with no entries and the default shrink-only baseline mode.
    pub(crate) const fn empty() -> Self {
        Self {
            append_only: Vec::new(),
            arith_files: Vec::new(),
            assertion_macros: Vec::new(),
            bans: Vec::new(),
            baseline: Vec::new(),
            baseline_mode: BaselineMode::ShrinkOnly,
            canary_dirs: Vec::new(),
            cargo_exceptions: Vec::new(),
            clippy_canaries: None,
            code: Vec::new(),
            default_range: None,
            fn_name_suffixes: Vec::new(),
            forbidden_dependencies: Vec::new(),
            forbidden_references: Vec::new(),
            judge: Vec::new(),
            macro_comments: Vec::new(),
            members: Vec::new(),
            silent_fallback_names: Vec::new(),
            test_attributes: Vec::new(),
        }
    }
}

/// A set of allowed keys.
struct KeySet(Vec<Name>);

impl KeySet {
    /// Builds a key set from string slices.
    fn of(items: &[&str]) -> Self {
        Self(items.iter().map(|item| Name::of(item)).collect())
    }
}

/// A view of the entries of one config table, with its location for errors.
struct Table<'ctx> {
    /// The key/value entries of the table.
    entries: &'ctx [Entry],
    /// The config file the table came from.
    file: &'ctx RelPath,
    /// Line at which the table starts, used in error messages.
    line: Line,
}

impl<'ctx> Table<'ctx> {
    /// Builds a parse failure at this table's location.
    fn fail(&self, what: &Message) -> Failure {
        Failure::parse_at(self.file, self.line, what)
    }

    /// Finds the entry whose single key equals `key`.
    #[must_use]
    fn find(&self, key: &Name) -> Option<&'ctx Entry> {
        self.entries
            .iter()
            .find(|entry| entry.single_key() == Some(key))
    }

    /// Reads the array of globs under `key`; empty if the key is absent. A text
    /// that is not a repo-relative glob fails at the line of the key.
    fn globs(&self, key: &Name) -> Result<Vec<Glob>, Failure> {
        let line = match self.find(key) {
            Some(entry) => entry.line,
            None => self.line,
        };
        self.list(key)?
            .iter()
            .map(|text| {
                Glob::parse(text, Brackets::Literal)
                    .map_err(|message| Failure::parse_at(self.file, line, &message))
            })
            .collect()
    }

    /// Fails if any entry's key is not in `keys`.
    fn known(&self, keys: &KeySet) -> Result<(), Failure> {
        for entry in self.entries {
            let ok = match entry.single_key() {
                Some(name) => keys.0.contains(name),
                None => false,
            };
            if !ok {
                return Err(Failure::parse_at(
                    self.file,
                    entry.line,
                    &Message::from_string(format!(
                        "unknown config key `{}`",
                        entry.key.dotted().as_str()
                    )),
                ));
            }
        }
        Ok(())
    }

    /// Reads the array of strings under `key`; empty if the key is absent.
    fn list(&self, key: &Name) -> Result<Vec<Text>, Failure> {
        match self.find(key) {
            None => Ok(Vec::new()),
            Some(entry) => strings(self.file, entry.line, &entry.value),
        }
    }

    /// Reads the string under `key`, failing if it is missing.
    fn need_text(&self, key: &Name) -> Result<Text, Failure> {
        match self.text(key)? {
            Some(text) => Ok(text),
            None => Err(self.fail(&Message::from_string(format!("missing `{}`", key.as_str())))),
        }
    }

    /// Reads the array of inline tables under `key`; empty if absent.
    fn tables(&self, key: &Name) -> Result<Vec<Self>, Failure> {
        match self.find(key) {
            None => Ok(Vec::new()),
            Some(entry) => inline_tables(self.file, entry.line, &entry.value),
        }
    }

    /// Reads the optional string under `key`, failing if it is not a string.
    fn text(&self, key: &Name) -> Result<Option<Text>, Failure> {
        match self.find(key) {
            None => Ok(None),
            Some(entry) => match entry.value.as_text() {
                Some(text) => Ok(Some(text.clone())),
                None => Err(Failure::parse_at(
                    self.file,
                    entry.line,
                    &Message::from_string(format!("`{}` must be a string", key.as_str())),
                )),
            },
        }
    }
}

/// Converts a value into a list of strings, failing otherwise.
fn strings(file: &RelPath, line: Line, value: &Value) -> Result<Vec<Text>, Failure> {
    let Some(items) = value.as_array() else {
        return Err(Failure::parse_at(
            file,
            line,
            &Message::of("expected an array of strings"),
        ));
    };
    items
        .iter()
        .map(|item| match item.as_text() {
            Some(text) => Ok(text.clone()),
            None => Err(Failure::parse_at(
                file,
                line,
                &Message::of("expected an array of strings"),
            )),
        })
        .collect()
}

/// Whether `text` is a crate name: ASCII letters, digits, `-` and `_`, at most
/// `MAX_CRATE_NAME` characters, and not empty.
fn is_crate_name(text: &Text) -> Answer {
    let name = text.as_str();
    let plain = name
        .chars()
        .all(|letter| letter.is_ascii_alphanumeric() || letter == '-' || letter == '_');
    if !name.is_empty() && name.len() <= MAX_CRATE_NAME && plain {
        Answer::Yes
    } else {
        Answer::No
    }
}

/// Reads one `[[forbidden-dependency]]` table.
fn forbidden_dependency(
    table: &Table<'_>,
    strictness: Strictness,
) -> Result<ForbiddenDependency, Failure> {
    table.known(&KeySet::of(&[
        "crate",
        "must-never-reach",
        "must-not-reach",
    ]))?;
    let crate_name = Name::from_string(table.need_text(&Name::of("crate"))?.as_str().to_owned());
    let must_not_reach = names(table.list(&Name::of("must-not-reach"))?);
    let never = table.list(&Name::of("must-never-reach"))?;
    for text in &never {
        if is_crate_name(text) == Answer::No {
            return Err(table.fail(&Message::from_string(format!(
                "[[forbidden-dependency]] for `{}`: `{}` is not a valid crate name in must-never-reach (ASCII letters, digits, `-` and `_`, at most {MAX_CRATE_NAME} characters)",
                crate_name.as_str(),
                text.as_str()
            ))));
        }
    }
    let must_never_reach = names(never);
    match strictness {
        Strictness::Required if must_not_reach.is_empty() && must_never_reach.is_empty() => {
            return Err(table.fail(&Message::from_string(format!(
                "[[forbidden-dependency]] for `{}` lists no must-not-reach or must-never-reach crate",
                crate_name.as_str()
            ))));
        }
        Strictness::Required | Strictness::Lenient => {}
    }
    Ok(ForbiddenDependency {
        crate_name,
        must_never_reach,
        must_not_reach,
    })
}

/// Converts a value into a list of tables, failing if it is not an array of tables.
fn inline_tables<'ctx>(
    file: &'ctx RelPath,
    line: Line,
    value: &'ctx Value,
) -> Result<Vec<Table<'ctx>>, Failure> {
    let Some(items) = value.as_array() else {
        return Err(Failure::parse_at(
            file,
            line,
            &Message::of("expected an array of inline tables"),
        ));
    };
    items
        .iter()
        .map(|item| match item.as_table() {
            Some(entries) => Ok(Table {
                entries,
                file,
                line,
            }),
            None => Err(Failure::parse_at(
                file,
                line,
                &Message::of("expected an array of inline tables"),
            )),
        })
        .collect()
}

/// Wraps each text as a name.
fn names(texts: Vec<Text>) -> Vec<Name> {
    texts
        .into_iter()
        .map(|text| Name::from_string(text.as_str().to_owned()))
        .collect()
}

/// Parses a config file's contents. `code` is required: without it
/// `judge-code-pairing` can never fire, and a pass that silently skipped a check
/// is what this tool exists to prevent.
pub(crate) fn parse(file: &RelPath, source: &Source) -> Result<Config, Failure> {
    let doc = toml::parse(file, source)?;
    from_doc(file, &doc, Strictness::Required)
}

/// Parses the config as it was at the base of a change range. A config from
/// before `code` was required parses with an empty `code`.
pub(crate) fn parse_base(file: &RelPath, source: &Source) -> Result<Config, Failure> {
    let doc = toml::parse(file, source)?;
    from_doc(file, &doc, Strictness::Lenient)
}

/// Builds a config from a parsed document, rejecting unknown sections.
fn from_doc(file: &RelPath, doc: &Doc, strictness: Strictness) -> Result<Config, Failure> {
    let mut config = Config::empty();
    for section in &doc.sections {
        let table = Table {
            entries: &section.entries,
            file,
            line: section.line,
        };
        let header: Vec<&str> = section.header.iter().map(Name::as_str).collect();
        match section.kind {
            SectionKind::Root => root_table(&table, &mut config, strictness)?,
            SectionKind::Table => match header.as_slice() {
                ["member", name] => config.members.push(member(&table, &Name::of(name))?),
                _ => return Err(unknown_section(file, section)),
            },
            SectionKind::ArrayOfTables => array_section(&table, section, &mut config, strictness)?,
        }
    }
    Ok(config)
}

/// Builds a parse failure for a section header that is not recognised.
fn unknown_section(file: &RelPath, section: &Section) -> Failure {
    let header: Vec<&str> = section.header.iter().map(Name::as_str).collect();
    Failure::parse_at(
        file,
        section.line,
        &Message::from_string(format!("unknown config section `{}`", header.join("."))),
    )
}

/// Reads a non-negative integer under `key`.
fn count_of(table: &Table<'_>, key: &Name) -> Result<Count, Failure> {
    let wrong = |what: &str| {
        table.fail(&Message::from_string(format!(
            "`{}` must {what}",
            key.as_str()
        )))
    };
    let Some(entry) = table.find(key) else {
        return Err(wrong("be an integer"));
    };
    match &entry.value {
        Value::Int(number) => match usize::try_from(*number) {
            Ok(value) => Ok(Count::new(value)),
            Err(_) => Err(wrong("not be negative")),
        },
        Value::Str(_) | Value::Bool(_) | Value::Raw(_) | Value::Array(_) | Value::Table(_) => {
            Err(wrong("be an integer"))
        }
    }
}

/// Handles one `[[...]]` section, adding its entry to `config`.
fn array_section(
    table: &Table<'_>,
    section: &Section,
    config: &mut Config,
    strictness: Strictness,
) -> Result<(), Failure> {
    let header: Vec<&str> = section.header.iter().map(Name::as_str).collect();
    match header.as_slice() {
        ["ban"] => {
            let added = ban(table)?;
            if config.bans.iter().any(|existing| existing.id == added.id) {
                return Err(table.fail(&Message::from_string(format!(
                    "duplicate [[ban]] id `{}`; each ban set needs its own id",
                    added.id.as_str()
                ))));
            }
            config.bans.push(added);
        }
        ["macro-comment"] => {
            table.known(&KeySet::of(&["macro", "prefix", "within-lines"]))?;
            config.macro_comments.push(MacroComment {
                macro_name: table.need_text(&Name::of("macro"))?,
                prefix: table.need_text(&Name::of("prefix"))?,
                within_lines: count_of(table, &Name::of("within-lines"))?,
            });
        }
        ["forbidden-dependency"] => {
            config
                .forbidden_dependencies
                .push(forbidden_dependency(table, strictness)?);
        }
        ["forbidden-reference"] => {
            table.known(&KeySet::of(&["files", "must-not-name"]))?;
            config.forbidden_references.push(ForbiddenReference {
                files: table.globs(&Name::of("files"))?,
                must_not_name: table.list(&Name::of("must-not-name"))?,
            });
        }
        ["baseline-entry"] => {
            table.known(&KeySet::of(&["rule", "file", "item", "index"]))?;
            config.baseline.push(BaselineEntry {
                file: RelPath::from_string(table.need_text(&Name::of("file"))?.as_str().to_owned()),
                index: count_of(table, &Name::of("index"))?,
                item: table.need_text(&Name::of("item"))?,
                line: section.line,
                rule: Name::from_string(table.need_text(&Name::of("rule"))?.as_str().to_owned()),
            });
        }
        _ => return Err(unknown_section(table.file, section)),
    }
    Ok(())
}

/// Reads the top-level keys of the config file into `config`.
fn root_table(
    table: &Table<'_>,
    config: &mut Config,
    strictness: Strictness,
) -> Result<(), Failure> {
    table.known(&KeySet::of(TOP_KEYS))?;
    config.baseline_mode = match table
        .text(&Name::of("baseline"))?
        .as_ref()
        .map(Text::as_str)
    {
        None | Some("shrink-only") => BaselineMode::ShrinkOnly,
        Some("forbidden") => BaselineMode::Forbidden,
        Some(_) => {
            return Err(table.fail(&Message::of(
                "`baseline` must be \"shrink-only\" or \"forbidden\"",
            )));
        }
    };
    config.default_range = table.text(&Name::of("default-range"))?;
    config.canary_dirs = table
        .list(&Name::of("canary-dirs"))?
        .into_iter()
        .map(|text| RelPath::from_string(text.as_str().to_owned()))
        .collect();
    for dir in &config.canary_dirs {
        let as_glob = Text::from_string(format!("{}/", dir.as_str().trim_end_matches('/')));
        Glob::parse(&as_glob, Brackets::Literal)
            .map(|_compiled| ())
            .map_err(|message| table.fail(&message))?;
    }
    if let Some(entry) = table.find(&Name::of("clippy-canaries")) {
        let Some(entries) = entry.value.as_table() else {
            return Err(table.fail(&Message::of("`clippy-canaries` must be an inline table")));
        };
        let inner = Table {
            entries,
            file: table.file,
            line: entry.line,
        };
        inner.known(&KeySet::of(&["package", "dir"]))?;
        config.clippy_canaries = Some(ClippyCanaries {
            dir: RelPath::from_string(inner.need_text(&Name::of("dir"))?.as_str().to_owned()),
            package: Name::from_string(inner.need_text(&Name::of("package"))?.as_str().to_owned()),
        });
    }
    config.arith_files = table.globs(&Name::of("arith-files"))?;
    config.test_attributes = table.list(&Name::of("test-attributes"))?;
    config.silent_fallback_names = names(table.list(&Name::of("silent-fallback-names"))?);
    config.fn_name_suffixes = names(table.list(&Name::of("fn-name-suffixes"))?);
    config.assertion_macros = names(table.list(&Name::of("assertion-macros"))?);
    config.judge = table.globs(&Name::of("judge"))?;
    config.code = table.globs(&Name::of("code"))?;
    config.append_only = table.globs(&Name::of("append-only"))?;
    match strictness {
        Strictness::Required if config.code.is_empty() => {
            return Err(table.fail(&Message::of(
                "`code` is required: list the globs of the code paths, such as code = [\"src/**\", \"tests/**\"]; without it judge-code-pairing never fires",
            )));
        }
        Strictness::Required | Strictness::Lenient => {}
    }
    config.cargo_exceptions = table.list(&Name::of("cargo-exceptions"))?;
    Ok(())
}

/// Builds the configuration of the member named `name` from its table.
fn member(table: &Table<'_>, name: &Name) -> Result<MemberConfig, Failure> {
    table.known(&KeySet::of(&["role", "ambient", "newtypes"]))?;
    let role = match table.need_text(&Name::of("role"))?.as_str() {
        "pure" => Role::Pure,
        "shell" => Role::Shell,
        _ => return Err(table.fail(&Message::of("`role` must be \"pure\" or \"shell\""))),
    };
    let mut ambient = Vec::new();
    for inner in table.tables(&Name::of("ambient"))? {
        inner.known(&KeySet::of(&["path", "files"]))?;
        ambient.push(AmbientAllow {
            files: inner.globs(&Name::of("files"))?,
            path: inner.need_text(&Name::of("path"))?,
        });
    }
    let mut newtypes = Vec::new();
    for inner in table.tables(&Name::of("newtypes"))? {
        inner.known(&KeySet::of(&["type", "methods"]))?;
        newtypes.push(NewtypeAllow {
            methods: names(inner.list(&Name::of("methods"))?),
            type_name: Name::from_string(inner.need_text(&Name::of("type"))?.as_str().to_owned()),
        });
    }
    Ok(MemberConfig {
        ambient,
        name: name.clone(),
        newtypes,
        role,
    })
}

/// Builds a ban set from one `[[ban]]` table.
fn ban(table: &Table<'_>) -> Result<BanSet, Failure> {
    table.known(&KeySet::of(&[
        "id",
        "files",
        "paths",
        "impl-traits",
        "derives",
        "macros",
        "methods",
    ]))?;
    let files = {
        let listed = table.globs(&Name::of("files"))?;
        if listed.is_empty() {
            vec![Glob::parse(&Text::of("**"), Brackets::Literal)
                .map_err(|message| table.fail(&message))?]
        } else {
            listed
        }
    };
    Ok(BanSet {
        derives: names(table.list(&Name::of("derives"))?),
        files,
        id: Name::from_string(table.need_text(&Name::of("id"))?.as_str().to_owned()),
        impl_traits: table.list(&Name::of("impl-traits"))?,
        macros: table.list(&Name::of("macros"))?,
        methods: names(table.list(&Name::of("methods"))?),
        paths: table.list(&Name::of("paths"))?,
    })
}

//! Canaries built into the binary: `self-test` proves the binary itself.
//! Each canary marks the lines that must fail with `//~ <rule-id>`.

use crate::canaries;
use crate::config::{BanSet, Config, ForbiddenReference, MacroComment};
use crate::failure::Failure;
use crate::findings::Finding;
use crate::glob::{Brackets, Glob};
use crate::types::{Count, Name, RelPath, Source, Text};
use alloc::collections::BTreeSet;

/// Built-in canaries as (name, source) pairs; each source marks expected findings with `//~`.
const CANARIES: &[(&str, &str)] = &[
    (
        "attributes_on_every_node",
        "// canary role=pure
pub struct S {
    #[allow(unused)] //~ suppression
    a: S,
}
pub enum E {
    #[allow(unused)] //~ suppression
    A,
}
pub trait T {
    #[allow(unused)] //~ suppression
    fn t(&self);
}
pub fn run<#[allow(unused)] P>( //~ suppression
    #[allow(unused)] //~ suppression
    p: P,
    #[allow(unused)] //~ suppression
    q: S,
) {
    #[allow(unused)] //~ suppression
    go();
    #[cfg(clippy)] //~ cfg-clippy
    go();
    #[ignore] //~ test-attributes
    go();
    #[allow(unused)] //~ suppression
    (go)();
    #[allow(unused)] //~ suppression
    x.go();
    #[allow(unused)] //~ suppression
    go!();
    #[allow(unused)] //~ suppression
    let f = || ();
    let g = #[allow(unused)] || (); //~ suppression
    let s = #[allow(unused)] S { //~ suppression
        #[allow(unused)] //~ suppression
        a: p,
    };
    #[allow(unused)] //~ suppression
    {
        go();
    }
    #[allow(unused)] //~ suppression
    if go() {}
    #[allow(unused)] //~ suppression
    match x {
        #[allow(unused)] //~ suppression
        K::A => (),
        K::B => (),
    }
    #[allow(unused)] //~ suppression
    let h = 1;
    #[allow(unused)] //~ suppression
    loop {}
    #[allow(unused)] //~ suppression
    for i in x {}
    #[allow(unused)] //~ suppression
    while go() {}
    #[allow(unused)] //~ suppression
    return;
}
",
    ),
    (
        "abort_unwrap",
        r#"// canary role=pure
pub struct W(Option<W>);
pub fn run(w: W) {
    w.0.unwrap(); //~ abort-unwrap
    w.0.expect("x"); //~ abort-unwrap
    Option::unwrap(w.0); //~ abort-unwrap
}
"#,
    ),
    (
        "abort_panic",
        r#"// canary role=pure
pub fn run() {
    panic!("x"); //~ abort-panic
    todo!(); //~ abort-panic
    std::process::exit(1); //~ abort-panic ambient-input
}
"#,
    ),
    (
        "abort_assert",
        "// canary role=pure
pub fn run() {
    assert!(true); //~ abort-assert
    debug_assert_eq!(1, 1); //~ abort-assert
}
",
    ),
    (
        "abort_assert_scope",
        "// canary role=pure
pub fn prod() { assert!(true); } //~ abort-assert
#[cfg(test)]
mod tests {
    pub fn helper() { assert!(true); }
    #[test]
    fn holds() { assert_eq!(1, 1); }
}
#[cfg(all(test, feature = \"x\"))]
mod more { pub fn helper() { assert_ne!(1, 2); } }
#[cfg(any(test, unix))]
mod maybe { pub fn helper() { assert!(true); } } //~ abort-assert
",
    ),
    (
        "forbidden_reference",
        "// canary role=pure
pub fn run() { let _a = crate::edit::Form::new(); } //~ forbidden-reference unused-underscore-binding
pub fn walk<T: crate::edit::Form>() {} //~ forbidden-reference
pub fn jog() { let _b = crate::edit::Form { id: 1 }; } //~ forbidden-reference unused-underscore-binding
pub fn lap() { let _c = crate::edit::Plain::new(); } //~ unused-underscore-binding
",
    ),
    (
        "silent_fallback",
        "// canary role=pure
pub struct W(Option<W>);
pub fn run(w: W) {
    w.0.unwrap_or(w); //~ silent-fallback
    w.0.map_or(w, w); //~ silent-fallback
    w.0.ok(); //~ silent-fallback
}
",
    ),
    (
        "discarded_value",
        "// canary role=pure
pub fn run() {
    let _ = run(); //~ discarded-value
    _ = run(); //~ discarded-value
}
",
    ),
    (
        "pub_fn_name",
        "// canary role=pure
pub fn run_raw() {} //~ pub-fn-name
fn run_unchecked() {} //~ pub-fn-name
",
    ),
    (
        "primitive_field",
        "// canary role=pure
pub struct S {
    pub n: u64, //~ primitive-field
    name: Option<String>, //~ primitive-field
}
",
    ),
    (
        "primitive_signature",
        "// canary role=pure
pub fn run(n: u64) {} //~ primitive-signature
pub fn walk() -> String { loop {} } //~ primitive-signature
pub fn glyph(c: char) {} //~ primitive-signature
pub fn truth() -> bool { loop {} } //~ primitive-signature
",
    ),
    (
        "bool_param",
        "// canary role=pure
pub fn run(flag: bool) {} //~ bool-param
pub fn walk() { let _f = |on: bool| (); } //~ bool-param unused-underscore-binding
",
    ),
    (
        "transposable_params",
        "// canary role=pure
pub struct A;
pub fn run(a: A, b: A) {} //~ transposable-params
",
    ),
    (
        "option_not_must_use",
        "// canary role=pure
pub struct A;
pub fn run() -> Option<A> { None } //~ option-not-must-use
",
    ),
    (
        "modular_arith",
        "// canary role=pure
pub struct W(W);
pub fn run(w: W) {
    w.0.wrapping_add(w.0); //~ modular-arith
    w.0.saturating_sub(w.0); //~ modular-arith
}
",
    ),
    (
        "arith_operator",
        "// canary role=pure
pub struct W(W);
pub fn run(w: W) {
    let x = w.0 + w.0; //~ arith-operator
    let y = -w.0; //~ arith-operator
}
",
    ),
    (
        "suppression",
        r#"// canary role=pure
#[allow(dead_code)] //~ suppression
pub fn run() {}
#[cfg_attr(feature = "x", warn(unused))] //~ suppression
pub fn walk() {}
#[expect(dead_code)] //~ suppression
pub fn jog() {}
"#,
    ),
    (
        "unordered_collection",
        "// canary role=pure
use std::collections::HashMap; //~ unordered-collection
use std::collections as c; // alias
pub fn run() { let _s: c::HashSet<()> = c::HashSet::new(); } //~ unordered-collection unused-underscore-binding
use hashbrown::HashMap as HM; //~ unordered-collection
pub fn fx() { let m = rustc_hash::FxHashMap::default(); } //~ unordered-collection
pub fn aset() { let m = ahash::AHashSet::new(); } //~ unordered-collection
pub fn hasher() { let m = std::collections::hash_map::DefaultHasher::new(); } //~ unordered-collection
pub fn ordered() { let m = indexmap::IndexMap::new(); }
",
    ),
    (
        "memory_leak",
        "// canary role=pure
use std::mem::ManuallyDrop; //~ memory-leak
use std::boxed::Box as B;
pub struct Item;
pub struct Held(ManuallyDrop<Item>); //~ memory-leak
pub fn boxed(item: Box<Item>) -> &'static mut Item { Box::leak(item) } //~ memory-leak
pub fn aliased(item: Box<Item>) -> &'static mut Item { B::leak(item) } //~ memory-leak
pub fn counted(item: std::ffi::OsString) { std::ffi::OsString::leak(item); } //~ memory-leak
pub fn vector(items: Vec<Item>) { Vec::leak(items); } //~ memory-leak
pub fn method(items: Vec<Item>) { items.leak(); } //~ memory-leak
pub fn core_path(item: Item) { core::mem::ManuallyDrop::new(item); } //~ memory-leak
pub fn forgotten(item: Item) { std::mem::forget(item); }
pub fn raw(item: Box<Item>) { Box::into_raw(item); }
",
    ),
    (
        "float_type",
        "// canary role=pure
pub struct W(f64); //~ float-type
pub fn run() { let x = 1.5; } //~ float-type
use std::f64::consts::E; //~ float-type
pub fn nan() { let a = f64::NAN; } //~ float-type
pub fn epsilon() { let a = std::f32::EPSILON; } //~ float-type
pub fn pi() { let a = core::f64::consts::PI; } //~ float-type
pub fn from() { let a = f64::from(1u8); } //~ float-type
pub fn bits() { let a = f64::from_bits(1u64); } //~ float-type
pub fn edge() { match 1u8 { f32::MAX => (), _ => () } } //~ float-type
",
    ),
    (
        "ambient_input",
        r#"// canary role=pure
pub fn run() {
    std::env::args(); //~ ambient-input
    println!("x"); //~ ambient-input
}
use std::os::unix::fs::symlink; //~ ambient-input
pub fn exists() { let a = std::path::Path::new("x").exists(); } //~ ambient-input
pub fn canonical() { let a = std::path::PathBuf::from("x").join("y").canonicalize(); } //~ ambient-input
pub fn qualified() { let a = std::path::Path::is_file(&p); } //~ ambient-input
pub fn user_defined(root: &R) { let a = root.exists(); }
"#,
    ),
    (
        "ambient_input_std_glob",
        "// canary role=pure
use std::*;
pub fn read() { io::stdin(); } //~ ambient-input
pub fn clock() { let a = time::SystemTime::now(); } //~ ambient-input
pub fn map() { let a = collections::HashMap::new(); } //~ unordered-collection
pub fn tidy() { let a = collections::BTreeMap::new(); }
",
    ),
    (
        "ambient_input_shell",
        r#"// canary role=shell
use std::fs as f; //~ ambient-input
pub fn run() { f::write("x", ""); } //~ ambient-input
"#,
    ),
    (
        "numeric_cast",
        "// canary role=pure
pub struct W(W);
pub fn run(w: W) {
    let x = w.0 as usize; //~ numeric-cast
    let y = w.0 as _; //~ numeric-cast
}
",
    ),
    (
        "catch_all_arm",
        "// canary role=pure
pub enum K { A, B }
pub fn run(k: K) {
    match k {
        K::A => (),
        _ => (), //~ catch-all-arm
    }
    match k {
        K::A => (),
        other => other, //~ catch-all-arm
    }
}
",
    ),
    (
        "block_scoped_enum",
        "// canary role=pure
pub fn run() {
    enum L { A, B }
    let l = L::A;
    match l {
        L::A => (),
        _ => (), //~ catch-all-arm
    }
    if let L::A = l {} //~ refutable-shortcut
    let b = matches!(l, L::B); //~ refutable-shortcut
    use L::*;
    match l {
        A => (),
        other => other, //~ catch-all-arm
    }
}
",
    ),
    (
        "refutable_shortcut",
        "// canary role=pure
pub enum K { A, B }
pub fn run(k: K) {
    if let K::A = k {} //~ refutable-shortcut
    let b = matches!(k, K::B); //~ refutable-shortcut
}
",
    ),
    (
        "struct_rest_pattern",
        "// canary role=pure
pub struct Config { pub a: Config, pub b: Config }
pub struct Wrapper(pub Config, pub Config);
pub enum Event { Move { x: Config, y: Config }, Pair(Config, Config), Quit }
pub enum Kind { A, B }
pub struct Tagged { pub kind: Kind, pub other: Kind }
pub fn lets(c: Config, w: Wrapper, e: Event) {
    let Config { a, .. } = c; //~ struct-rest-pattern
    let Wrapper(first, ..) = w; //~ struct-rest-pattern
    let Event::Move { x, .. } = e else { return }; //~ refutable-shortcut
}
pub fn arms(e: Event) {
    match e {
        Event::Move { x, .. } => (), //~ struct-rest-pattern
        Event::Pair(a, ..) => (), //~ struct-rest-pattern
        Event::Quit => (),
    }
}
pub fn others(e: Event, o: Option<Config>, cs: Vec<Config>) {
    if let Some(Config { a, .. }) = o {} //~ struct-rest-pattern
    for Config { a, .. } in cs {} //~ struct-rest-pattern
    let hit = matches!(e, Event::Move { .. }); //~ refutable-shortcut
}
pub fn param(Config { a, .. }: Config) {} //~ struct-rest-pattern
impl Config {
    pub fn first(self) -> Config { let Self { a, .. } = self; a } //~ struct-rest-pattern
}
macro_rules! take { ($c:expr) => { let Config { a, .. } = $c; }; } //~ struct-rest-pattern unparsed-macro
pub fn owned_by_catch_all(t: Tagged) {
    match t {
        Tagged { kind: Kind::A, .. } => (), //~ catch-all-arm
        Tagged { kind: Kind::B, .. } => (), //~ catch-all-arm
    }
}
pub struct Unit;
pub fn fieldless(u: Unit) {
    let Unit { .. } = u;
}
pub fn not_flagged(m: syn::MetaList, s: &[Config], c: Config) {
    let syn::MetaList { path, .. } = m;
    let [first, ..] = s;
    let Config { a, b: _ } = c;
}
",
    ),
    (
        "default_construction",
        "// canary role=pure
pub struct Item;
#[derive(Debug, Default)] //~ default-construction
pub struct Plain { pub a: Item }
#[derive(Default)] //~ default-construction
pub union Raw { pub a: Item }
#[cfg_attr(feature = \"x\", derive(Default))] //~ default-construction
pub struct Gated { pub a: Item }
#[derive(Default)]
pub enum Mode { #[default] Fast, Slow }
macro_rules! make { () => { #[derive(Default)] pub struct Made { pub a: Item } }; } //~ default-construction unparsed-macro
pub struct Pair { pub a: Item, pub b: Item }
pub fn updates(a: Item, old: Pair) -> Pair {
    let first = Pair { a, ..Default::default() }; //~ default-construction
    let second = Pair { a: Item, ..Pair::default() }; //~ default-construction
    let third = Pair { a: Item, ..old };
    let fourth = syn::MetaList { path: Item, ..Default::default() };
    first
}
",
    ),
    (
        "unused_underscore_binding",
        "// canary role=pure
pub struct Item;
pub fn fallible() -> Result<Item, Item> { Ok(Item) }
pub fn consume<T>(value: T) { drop(value); }
pub fn flagged() {
    let _result = fallible(); //~ unused-underscore-binding
    let (kept, _second) = (Item, fallible()); //~ unused-underscore-binding
    let _guard = fallible(); //~ unused-underscore-binding
}
pub fn reads() {
    let _a = fallible();
    consume(_a);
    let _c = Item;
    let text = format!(\"{_c:?}\");
    let _guard = fallible();
    drop(_guard);
}
",
    ),
    (
        "dropped_call_result",
        "// canary role=pure
use std::mem::drop as release;
pub struct Item;
pub fn fallible() -> Result<Item, Item> { Ok(Item) }
pub fn flagged() {
    drop(fallible()); //~ dropped-call-result
    std::mem::drop(fallible()); //~ dropped-call-result
    release(fallible()); //~ dropped-call-result
    let guard = fallible();
    drop(guard);
}
",
    ),
    (
        "swallowed_error",
        "// canary role=pure
pub struct Item;
pub fn fallible() -> Result<Item, Item> { Ok(Item) }
pub fn consume<T>(value: T) { drop(value); }
pub fn flagged(result: Result<Item, Item>) {
    match result { Ok(item) => consume(item), Err(_) => () } //~ swallowed-error
    if let Ok(item) = fallible() { consume(item); } //~ swallowed-error
    while let Ok(item) = fallible() { consume(item); } //~ swallowed-error
    match result { Ok(item) => consume(item), Err(reason) => consume(reason) }
    if let Ok(item) = fallible() { consume(item); } else { consume(Item); }
}
",
    ),
    (
        "cfg_clippy",
        "// canary role=pure
#[cfg(clippy)] //~ cfg-clippy
pub fn run() {}
#[cfg_attr(clippy, derive(Debug))] //~ cfg-clippy
pub struct S;
",
    ),
    (
        "test_assertion",
        "// canary role=pure
#[test]
fn t() {} //~ test-assertion
",
    ),
    (
        "test_early_return",
        "// canary role=pure
pub struct E;
#[test]
fn bare() {
    return; //~ test-early-return
    assert!(true);
}
#[test]
fn ok_unit() -> Result<(), E> {
    return Ok(()); //~ test-early-return
    assert!(true);
}
#[test]
fn nested() {
    if go() {
        return; //~ test-early-return
    }
    assert!(true);
}
#[test]
fn let_else() {
    let Some(x) = go() else { //~ test-early-return
        return;
    };
    assert!(true);
}
#[test]
fn err_spared() -> Result<(), E> {
    if go() {
        return Err(E);
    }
    assert!(true);
    Ok(())
}
#[test]
fn ok_flagged() -> Result<(), E> {
    return Ok(()); //~ test-early-return
    assert!(true);
}
#[test]
fn spared() {
    let f = || {
        return;
    };
    fn inner() {
        return;
    }
    assert!(true);
    return;
}
",
    ),
    (
        "test_attributes",
        "// canary role=pure
#[test]
#[ignore] //~ test-attributes
fn t() { assert!(true); }
#[test]
#[should_panic] //~ test-attributes
fn u() { assert!(true); }
",
    ),
    (
        "macro_needs_comment",
        "// canary role=pure
proptest! { #[test] fn p(x in 0..3u8) { prop_assert!(x < 3); } } //~ macro-needs-comment
",
    ),
    (
        "unscannable_include",
        r#"// canary role=pure
include!(concat!("a", "b")); //~ unscannable-include
"#,
    ),
    (
        "unparsed_macro",
        "// canary role=pure
macro_rules! m { () => { x.unwrap() } } //~ abort-unwrap unparsed-macro
macro_rules! n { //~ unparsed-macro
    ($($v:expr),*) => {
        $( $v.unwrap(); )* //~ abort-unwrap
    };
    ($t:ty) => {
        fn make() -> $t { Default::default() }
        pub fn big() { let _x = 1.5; } //~ float-type unused-underscore-binding
        pub fn raw() { panic!(\"x\"); } //~ abort-panic
    };
}
macro_rules! plain { () => { 1 } }
weird!(=> ;; x y ,, =>); //~ unparsed-macro
Token![,];
",
    ),
    (
        "banned",
        "// canary role=pure
pub fn run() { std::mem::swap(); } //~ banned:selftest-ban
",
    ),
];

/// The config the built-in canaries run under.
fn config() -> Result<Config, Failure> {
    let everything = Glob::parse(&Text::of("**"), Brackets::Literal)
        .map_err(|message| Failure::internal(&message))?;
    let mut config = Config::empty();
    config.bans.push(BanSet {
        derives: Vec::new(),
        files: vec![everything.clone()],
        id: Name::of("selftest-ban"),
        impl_traits: Vec::new(),
        macros: Vec::new(),
        methods: Vec::new(),
        paths: vec![Text::from_string("std::mem::swap".to_owned())],
    });
    config.forbidden_references.push(ForbiddenReference {
        files: vec![everything],
        must_not_name: vec![Text::from_string("crate::edit::Form".to_owned())],
    });
    config.macro_comments.push(MacroComment {
        macro_name: Text::from_string("proptest".to_owned()),
        prefix: Text::from_string("// pins: ".to_owned()),
        within_lines: Count::new(3),
    });
    Ok(config)
}

/// Runs every built-in canary and checks that every rule is covered.
pub(crate) fn run() -> Result<Vec<Finding>, Failure> {
    let config = config()?;
    let mut findings = Vec::new();
    let mut covered = BTreeSet::new();
    for (name, text) in CANARIES {
        let path = RelPath::of(&format!("<self-test>/{name}.rs"));
        let mut result =
            canaries::check_rule_canary(&config, &path, &Source::from_string((*text).to_owned()))?;
        findings.append(&mut result.findings);
        covered.extend(result.ids);
    }
    findings.extend(canaries::coverage_findings(
        &config,
        &covered,
        &RelPath::of("<self-test>"),
    ));
    Ok(findings)
}

# Canary exemptions

## Canary exemptions

| Lint | Why it cannot be triggered |
| --- | --- |
| `aarch64_softfloat_neon` | Needs an aarch64 softfloat target (aarch64-unknown-none-softfloat) |
| `absolute_paths_not_starting_with_crate` | Only fires in edition 2015; canary crate is edition 2021 where ::m::f() is a hard error E0433 |
| `ambiguous_derive_helpers` | Needs a proc-macro derive crate (extra dependency); a bin crate cannot define or use one without Cargo.toml changes |
| `ambiguous_glob_imports` | In rustc 1.98 every tried pattern (incl. the lint's own doc example) is a hard error E0659 rather than the lint |
| `ambiguous_import_visibilities` | Every tried pattern gives hard error E0659 (or no diagnostic) in rustc 1.98 instead of the lint |
| `anonymous_parameters` | Only fires in edition 2015; trait methods without parameter names are a hard error in edition 2021 |
| `array_into_iter` | Only fires for editions before 2021; array.into_iter() by-value is the normal behavior in 2021 |
| `asm_sub_register` | Target-specific (needs asm! with a register class on x86/aarch64) and asm! needs unsafe; not portable across targets |
| `bad_asm_style` | x86/x86_64 only (AT&T vs Intel syntax asm), not triggerable on aarch64 |
| `bare_trait_objects` | Edition 2021 turns bare trait objects into hard error E0782 instead of the lint |
| `binary_asm_labels` | x86/x86_64 only (binary-looking asm labels) |
| `bindings_with_variant_name` | Emitted with diagnostic code E0170 rather than the lint name, so code-based verification cannot match |
| `clippy::await_holding_invalid_type` | fires only for types listed in the clippy.toml knob await-holding-invalid-types (default empty), which this slice may not edit |
| `clippy::cargo_common_metadata` | cargo group lint about missing Cargo.toml package metadata; its diagnostic points at the manifest, not a Rust source line, so it cannot be marked with //~ |
| `clippy::cast_slice_from_raw_parts` | fires only on slice::from_raw_parts(..) casts, which require an unsafe block, and the workspace sets unsafe_code = "forbid" |
| `clippy::crosspointer_transmute` | requires mem::transmute, which needs an unsafe block, and the workspace sets unsafe_code = "forbid" |
| `clippy::disallowed_fields` | Only fires for fields listed in the disallowed-fields clippy.toml knob, which is not set. |
| `clippy::disallowed_macros` | Only fires for macros listed in the disallowed-macros clippy.toml knob, which is not set. |
| `clippy::disallowed_methods` | Only fires for methods listed in the disallowed-methods clippy.toml knob, which is not set. |
| `clippy::disallowed_types` | Only fires for types listed in the disallowed-types clippy.toml knob, which is not set. |
| `clippy::empty_enums` | Only fires when the unstable never_type feature is enabled (nightly-only). |
| `clippy::enum_clike_unportable_variant` | Only fires when compiling for a 32-bit target (discriminant overflows 32-bit isize); not triggerable on a 64-bit host. |
| `clippy::excessive_nesting` | Disabled by default (excessive-nesting-threshold = 0); only fires when that clippy.toml knob is set, which it is not. |
| `clippy::inefficient_to_string` | Never fires on rustc/clippy 1.98.1: tried .to_string() on &&str, &&String, &&char, &&bool, &&Cow<str>, &&&str; the std ToString specialisation now covers these, so the lint is effectively dead. |
| `clippy::inline_asm_x86_att_syntax` | Needs asm!/global_asm!, which requires unsafe (workspace has unsafe_code = "forbid", which cannot be overridden) and an x86/x86_64 target. |
| `clippy::inline_asm_x86_intel_syntax` | Needs asm!/global_asm!, which requires unsafe (workspace has unsafe_code = "forbid", which cannot be overridden) and an x86/x86_64 target. |
| `clippy::invalid_regex` | Only fires on regex crate constructors (Regex::new, RegexSet::new, ...); the canary crate has no dependencies and Cargo.toml may not be edited. |
| `clippy::large_include_file` | Needs include_str!/include_bytes! of a file larger than 1,000,000 bytes (max-include-file-size knob); only files under canaries/rust/src/bin/ may be added and none may be that large. |
| `clippy::let_underscore_lock` | Superseded by the rustc lint let_underscore_lock; on 1.98.1 let _ = mutex.lock() only produces the rustc diagnostic, never the clippy:: one (tried Result and guard-typed initialisers). |
| `clippy::lint_groups_priority` | Only checks the [lints]/[workspace.lints] tables in Cargo.toml (group vs lint priority); it is a manifest condition and cannot fire from a .rs file. |
| `clippy::macro_use_imports` | Does not fire on 1.98.1 without an external macro crate: tried #[macro_use] use (rustc rejects with unused_attributes) and #[macro_use] extern crate of core/alloc/std; the canary crate has no dependencies. |
| `clippy::manual_noop_waker` | Fires on a hand-built Waker::from_raw(RawWaker::new(..)), which is an unsafe call that unsafe_code = "forbid" rejects. |
| `clippy::mem_replace_with_uninit` | Needs mem::uninitialized()/mem::zeroed() (unsafe, plus deprecated), forbidden by unsafe_code = "forbid". |
| `clippy::missing_enforced_import_renames` | Only fires when clippy.toml enforced-import-renames is configured, and it is not (knob-only lint). |
| `clippy::missing_safety_doc` | Needs an unsafe fn/unsafe trait, but the workspace sets unsafe_code = "forbid", so any canary errors out. |
| `clippy::multiple_crate_versions` | cargo-group manifest lint; needs duplicate dependency versions in Cargo.lock/Cargo.toml, which canary sources cannot create |
| `clippy::negative_feature_names` | cargo-group manifest lint on Cargo.toml feature names; cannot be triggered from a .rs source file |
| `clippy::non_std_lazy_statics` | only fires on use of external lazy_static/once_cell crates, which the canary crate does not depend on |
| `clippy::nonstandard_macro_braces` | fires only for macros configured via the clippy.toml standard-macro-braces knob, which is not set |
| `clippy::pointers_in_nomem_asm_block` | Requires an asm! block (unsafe, unstable-arch specific); workspace sets unsafe_code = "forbid" |
| `clippy::redundant_feature_names` | cargo group lint: fires only on a Cargo.toml manifest condition (feature named like use-x/with-x), not on source |
| `clippy::redundant_test_prefix` | Only fires on #[test] fns, which exist only under cfg(test); the verification command (cargo clippy --bin NAME) never compiles test cfg |
| `clippy::ref_binding_to_reference` | Still registered, but in clippy 1.98 its pattern check was folded into needless_borrow: every ref-to-reference pattern is reported with code clippy::needless_borrow ("this pattern creates a reference to a reference"), never with this code (verified with -W on many shapes) |
| `clippy::regex_creation_in_loops` | Requires calling regex::Regex::new in a loop; the canary crate has no regex dependency (and Cargo.toml may not be edited) |
| `clippy::same_length_and_capacity` | Requires an unsafe call (String/Vec::from_raw_parts); workspace sets unsafe_code = "forbid" |
| `clippy::self_named_module_files` | File-layout lint: needs a mod.rs file, but canaries must be a single src/bin/<name>.rs file |
| `clippy::serde_api_misuse` | Requires the serde crate's API (e.g. a Deserialize/Serialize impl); the canary crate has no serde dependency (and Cargo.toml may not be edited) |
| `clippy::strlen_on_c_strings` | Only matches a call to libc::strlen (the external libc crate), and the canary crate has no libc dependency that can be added here. |
| `clippy::suspicious_else_formatting` | Only fires on else followed by a newline then the block/if, with the primary span starting on the else line; any trailing //~ marker on that line sits between else and the block and suppresses the lint, so the line cannot carry a mark (} { form is the separate possible_missing_else lint). |
| `clippy::tests_outside_test_module` | Only fires on functions carrying #[test], which rustc strips unless compiled with --test/cfg(test); the verification command (cargo clippy --bin) never enables cfg(test). |
| `clippy::toplevel_ref_arg` | Suppressed whenever clippy::ref_patterns is enabled (the workspace denies it, so the ref pattern is reported only as ref_patterns); verified it fires only with ref_patterns allowed. |
| `clippy::trivial_regex` | Only matches regex::Regex::new/RegexBuilder::new from the external regex crate, which the canary crate does not depend on. |
| `clippy::unicode_not_nfc` | Only reported when clippy::non_ascii_literal is not enabled (it is an else-branch of that check), and the workspace denies non_ascii_literal; verified silent with both enabled. |
| `clippy::unsafe_derive_deserialize` | needs serde::Deserialize derive on a type with unsafe methods; serde is not a dependency of the canary crate and unsafe_code is forbidden |
| `clippy::wildcard_dependencies` | cargo-group lint that inspects Cargo.toml manifest dependencies (a * version requirement), not source code |
| `conflicting_repr_hints` | Emitted with diagnostic code E0566 rather than the lint name, so code-based verification cannot match |
| `duplicate_features` | #![feature(..)] is rejected on stable (E0554) |
| `ellipsis_inclusive_range_patterns` | ... range patterns are a hard error (E0783) in edition 2021 |
| `explicit_builtin_cfgs_in_flags` | Triggered only by --cfg command-line flags, not by source code |
| `exported_private_dependencies` | Needs an extra Cargo dependency and a library crate's public API; not possible in a dependency-free bin crate |
| `hidden_glob_reexports` | Only fires for externally reachable (lib crate) public glob re-exports; does not fire in a binary crate |
| `impl_trait_redundant_captures` | Only fires for edition 2024 implicit captures; no diagnostic in edition 2021 |
| `improper_gpu_kernel_arg` | Needs unstable extern "gpu-kernel" ABI (nightly) |
| `incomplete_features` | Needs #![feature(..)] (nightly) |
| `incomplete_include` | Primary span lands in the included file, not on the marked line of the canary |
| `ineffective_unstable_trait_impl` | Needs #[unstable] stability attributes (staged_api, nightly) |
| `inline_no_sanitize` | Needs unstable #[no_sanitize]/sanitize attribute feature (nightly) |
| `internal_features` | Needs #![feature(rustc_attrs)] or similar internal features (nightly) |
| `keyword_idents_2018` | Only fires for edition 2015 crates using async/await/try/dyn as identifiers |
| `large_assignments` | Needs unstable large_assignments feature (#![move_size_limit], nightly) |
| `legacy_derive_helpers` | Needs a proc-macro derive crate (extra dependency) |
| `linker_info` | Needs informational linker output, not produced by cargo clippy checks |
| `linker_messages` | Needs linker warnings at link time, not produced by cargo clippy checks |
| `missing_gpu_kernel_export_name` | Needs unstable extern "gpu-kernel" ABI (nightly) |
| `named_asm_labels` | Needs an asm! block, and on x86/x86_64 (x86_64-unknown-linux-gnu, x86_64-apple-darwin) clippy adds `clippy::inline_asm_x86_intel_syntax` (or `inline_asm_x86_att_syntax` with `options(att_syntax)`, both denied) to every asm! block, so no block gives the same diagnostics on aarch64 and on x86; found by a CI run on x86_64 Linux. |
| `non_fmt_panics` | Only fires in edition 2015/2018; the canary crate is edition 2021 where panic!("{}") is a hard error. |
| `opaque_hidden_inferred_bound` | Tried the documented nested impl Trait example on rustc 1.98.1; no diagnostic is emitted. |
| `out_of_scope_macro_calls` | Documented trigger (#[doc = in_root!()] before the macro) is now a hard error "cannot find macro" with no lint code. |
| `proc_macro_derive_resolution_fallback` | Only relevant to proc-macro crates (manifest proc-macro = true); not triggerable in a plain binary. |
| `pub_use_of_private_extern_crate` | Emitted with rustc error code E0365 (pub use of a private extern crate), never with the lint name. |
| `rtsan_nonblocking_async` | Needs the unstable #[sanitize(realtime = ...)] attribute; also emits E0658 on stable. |
| `rust_2021_incompatible_closure_captures` | Edition-migration lint that only fires in editions before 2021; canary crate is edition 2021. |
| `rust_2021_incompatible_or_patterns` | Edition-migration lint that only fires in editions before 2021; canary crate is edition 2021. |
| `rust_2021_prefixes_incompatible_syntax` | Edition-migration lint that only fires in editions before 2021; canary crate is edition 2021. |
| `rust_2021_prelude_collisions` | Edition-migration lint that only fires in editions before 2021; canary crate is edition 2021. |
| `special_module_name` | Only fires for a file-based mod lib;/mod main; resolving to a sibling lib.rs/main.rs, which would be another bin target; inline or #[path] modules do not trigger it. |
| `stable_features` | Requires #![feature(...)], which on stable also emits E0554. |
| `trivial_bounds` | Needs #![feature(trivial_bounds)]; on stable false trivial bounds are hard errors (E0277) and true ones emit nothing. |
| `tyvar_behind_raw_pointer` | Only fires in edition 2015/2018; it is a hard error in edition 2021. |
| `uncovered_param_in_projection` | Single-crate attempts only produce hard errors (E0207/E0210/E0119); the lint needs a foreign crate's trait/type. |
| `unnameable_test_items` | Concerns #[test_case] items, which require the unstable custom_test_frameworks feature. |
| `unsafe_op_in_unsafe_fn` | On this rustc the diagnostic is reported with error code E0133, never with the lint name. |
| `unstable_features` | Needs #![feature(...)], which fails with E0554 on stable. |
| `unused_crate_dependencies` | Needs an unused dependency in the canary crate's Cargo.toml, which has none and must not be edited. |
| `unused_features` | Needs #![feature(...)], which fails with E0554 on stable. |
| `uses_power_alignment` | Only fires for #[repr(C)] types on AIX/PowerPC targets, not on this host. |
| `varargs_without_pattern` | Needs unstable c_variadic function definitions; emits E0658 on stable. |
| `warnings` | A lint group alias for all warnings; no diagnostic ever carries the code warnings. |
| `clippy::items_after_test_module` | it only fires for a `#[cfg(test)]` module followed by items in a test-gated crate; the canary binary never compiles test code. |

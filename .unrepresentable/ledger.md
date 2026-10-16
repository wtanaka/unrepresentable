# Unrepresentable ledgers

## Expect ledger

| File | Item | Lint | Reason |
| --- | --- | --- | --- |
| — | — | — | — |

## Lint omissions

| Lint | Contradicts | Reason |
| --- | --- | --- |
| `clippy::pub_with_shorthand` | `clippy::pub_without_shorthand` | `pub(in crate)` and `pub(crate)` are mutually exclusive spellings; the crate uses `pub(crate)`. |
| `clippy::mod_module_files` | `clippy::self_named_module_files` | `mod.rs` and `name.rs` + `name/` are mutually exclusive layouts; the crate uses the second. |
| `clippy::implicit_return` | `clippy::needless_return` | implicit_return requires an explicit `return` on every tail expression, which needless_return forbids. |
| `clippy::question_mark_used` | `clippy::question_mark` | question_mark rewrites early-return patterns into `?`, while question_mark_used forbids `?`; abort-unwrap and silent-fallback leave `?` as the only propagation. |
| `clippy::redundant_pub_crate` | `unreachable_pub` | in a private module `pub` is unreachable (unreachable_pub) and `pub(crate)` is redundant (redundant_pub_crate); the crate keeps `pub(crate)`. |
| `clippy::single_call_fn` | `clippy::too_many_lines` | splitting long functions to satisfy too_many_lines produces helpers with a single caller. |
| `clippy::pattern_type_mismatch` | `clippy::ref_patterns` | matching through references needs either `ref` bindings (banned by ref_patterns) or explicit `&` patterns that cannot bind non-Copy fields. |
| `clippy::missing_trait_methods` | - | syn::visit::Visit has about two hundred provided methods; naming each one adds no safety. |
| `clippy::renamed_function_params` | `clippy::min_ident_chars` | trait impls must reuse the trait's parameter names, and syn's `Visit` methods name theirs `i` and `node`, which min_ident_chars forbids. |
| `clippy::option_if_let_else` | - | it requires `map_or` and `map_or_else`, which this tool's own silent-fallback rule bans; a contradiction with a tool rule rather than a lint. |
| `clippy::wildcard_enum_match_arm` | - | syn's enums are `#[non_exhaustive]`, so a wildcard arm is required and cannot name every variant. |
| `default_overrides_default_fields` | unstable | unstable on the pinned stable toolchain: setting it is an `unknown lint` error. |
| `deprecated_llvm_intrinsic` | unstable | unstable on the pinned stable toolchain: setting it is an `unknown lint` error. |
| `implicit_provenance_casts` | unstable | unstable on the pinned stable toolchain: setting it is an `unknown lint` error. |
| `multiple_supertrait_upcastable` | unstable | unstable on the pinned stable toolchain: setting it is an `unknown lint` error. |
| `must_not_suspend` | unstable | unstable on the pinned stable toolchain: setting it is an `unknown lint` error. |
| `non_exhaustive_omitted_patterns` | unstable | unstable on the pinned stable toolchain: setting it is an `unknown lint` error. |
| `resolving_to_items_shadowing_supertrait_items` | unstable | unstable on the pinned stable toolchain: setting it is an `unknown lint` error. |
| `shadowing_supertrait_items` | unstable | unstable on the pinned stable toolchain: setting it is an `unknown lint` error. |
| `tail_call_track_caller` | unstable | unstable on the pinned stable toolchain: setting it is an `unknown lint` error. |
| `test_unstable_lint` | unstable | unstable on the pinned stable toolchain: setting it is an `unknown lint` error. |
| `unqualified_local_imports` | unstable | unstable on the pinned stable toolchain: setting it is an `unknown lint` error. |

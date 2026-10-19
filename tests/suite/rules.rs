//! One positive and one negative fixture per rule; expected findings live in `tests/expected/`.

use crate::support::{check_case, Name};

#[test]
fn clean_positive() {
    assert_eq!(
        check_case(&Name::of("clean/positive")),
        Ok(()),
        "clean/positive"
    );
}

#[test]
fn abort_unwrap_positive() {
    assert_eq!(
        check_case(&Name::of("abort-unwrap/positive")),
        Ok(()),
        "abort-unwrap/positive"
    );
}

#[test]
fn abort_unwrap_negative() {
    assert_eq!(
        check_case(&Name::of("abort-unwrap/negative")),
        Ok(()),
        "abort-unwrap/negative"
    );
}

#[test]
fn abort_panic_positive() {
    assert_eq!(
        check_case(&Name::of("abort-panic/positive")),
        Ok(()),
        "abort-panic/positive"
    );
}

#[test]
fn abort_panic_negative() {
    assert_eq!(
        check_case(&Name::of("abort-panic/negative")),
        Ok(()),
        "abort-panic/negative"
    );
}

#[test]
fn abort_assert_positive() {
    assert_eq!(
        check_case(&Name::of("abort-assert/positive")),
        Ok(()),
        "abort-assert/positive"
    );
}

#[test]
fn abort_assert_negative() {
    assert_eq!(
        check_case(&Name::of("abort-assert/negative")),
        Ok(()),
        "abort-assert/negative"
    );
}

#[test]
fn silent_fallback_positive() {
    assert_eq!(
        check_case(&Name::of("silent-fallback/positive")),
        Ok(()),
        "silent-fallback/positive"
    );
}

#[test]
fn silent_fallback_negative() {
    assert_eq!(
        check_case(&Name::of("silent-fallback/negative")),
        Ok(()),
        "silent-fallback/negative"
    );
}

#[test]
fn discarded_value_positive() {
    assert_eq!(
        check_case(&Name::of("discarded-value/positive")),
        Ok(()),
        "discarded-value/positive"
    );
}

#[test]
fn discarded_value_negative() {
    assert_eq!(
        check_case(&Name::of("discarded-value/negative")),
        Ok(()),
        "discarded-value/negative"
    );
}

#[test]
fn pub_fn_name_positive() {
    assert_eq!(
        check_case(&Name::of("pub-fn-name/positive")),
        Ok(()),
        "pub-fn-name/positive"
    );
}

#[test]
fn pub_fn_name_configured() {
    assert_eq!(
        check_case(&Name::of("pub-fn-name/configured")),
        Ok(()),
        "pub-fn-name/configured"
    );
}

#[test]
fn pub_fn_name_negative() {
    assert_eq!(
        check_case(&Name::of("pub-fn-name/negative")),
        Ok(()),
        "pub-fn-name/negative"
    );
}

#[test]
fn primitive_field_positive() {
    assert_eq!(
        check_case(&Name::of("primitive-field/positive")),
        Ok(()),
        "primitive-field/positive"
    );
}

#[test]
fn primitive_field_negative() {
    assert_eq!(
        check_case(&Name::of("primitive-field/negative")),
        Ok(()),
        "primitive-field/negative"
    );
}

#[test]
fn primitive_signature_positive() {
    assert_eq!(
        check_case(&Name::of("primitive-signature/positive")),
        Ok(()),
        "primitive-signature/positive"
    );
}

#[test]
fn primitive_signature_negative() {
    assert_eq!(
        check_case(&Name::of("primitive-signature/negative")),
        Ok(()),
        "primitive-signature/negative"
    );
}

#[test]
fn primitive_signature_newtype_methods() {
    assert_eq!(
        check_case(&Name::of("primitive-signature/newtype-methods")),
        Ok(()),
        "primitive-signature/newtype-methods"
    );
}

#[test]
fn primitive_signature_char_and_bool_return() {
    assert_eq!(
        check_case(&Name::of("primitive-signature/char-and-bool-return")),
        Ok(()),
        "primitive-signature/char-and-bool-return"
    );
}

#[test]
fn bool_param_positive() {
    assert_eq!(
        check_case(&Name::of("bool-param/positive")),
        Ok(()),
        "bool-param/positive"
    );
}

#[test]
fn bool_param_negative() {
    assert_eq!(
        check_case(&Name::of("bool-param/negative")),
        Ok(()),
        "bool-param/negative"
    );
}

#[test]
fn transposable_params_positive() {
    assert_eq!(
        check_case(&Name::of("transposable-params/positive")),
        Ok(()),
        "transposable-params/positive"
    );
}

#[test]
fn transposable_params_negative() {
    assert_eq!(
        check_case(&Name::of("transposable-params/negative")),
        Ok(()),
        "transposable-params/negative"
    );
}

#[test]
fn option_not_must_use_positive() {
    assert_eq!(
        check_case(&Name::of("option-not-must-use/positive")),
        Ok(()),
        "option-not-must-use/positive"
    );
}

#[test]
fn option_not_must_use_negative() {
    assert_eq!(
        check_case(&Name::of("option-not-must-use/negative")),
        Ok(()),
        "option-not-must-use/negative"
    );
}

#[test]
fn modular_arith_positive() {
    assert_eq!(
        check_case(&Name::of("modular-arith/positive")),
        Ok(()),
        "modular-arith/positive"
    );
}

#[test]
fn modular_arith_negative() {
    assert_eq!(
        check_case(&Name::of("modular-arith/negative")),
        Ok(()),
        "modular-arith/negative"
    );
}

#[test]
fn arith_operator_positive() {
    assert_eq!(
        check_case(&Name::of("arith-operator/positive")),
        Ok(()),
        "arith-operator/positive"
    );
}

#[test]
fn arith_operator_negative() {
    assert_eq!(
        check_case(&Name::of("arith-operator/negative")),
        Ok(()),
        "arith-operator/negative"
    );
}

#[test]
fn arith_operator_arith_file() {
    assert_eq!(
        check_case(&Name::of("arith-operator/arith-file")),
        Ok(()),
        "arith-operator/arith-file"
    );
}

#[test]
fn arith_operator_outside_arith_file() {
    assert_eq!(
        check_case(&Name::of("arith-operator/outside-arith-file")),
        Ok(()),
        "arith-operator/outside-arith-file"
    );
}

#[test]
fn suppression_positive() {
    assert_eq!(
        check_case(&Name::of("suppression/positive")),
        Ok(()),
        "suppression/positive"
    );
}

#[test]
fn suppression_negative() {
    assert_eq!(
        check_case(&Name::of("suppression/negative")),
        Ok(()),
        "suppression/negative"
    );
}

#[test]
fn unordered_collection_positive() {
    assert_eq!(
        check_case(&Name::of("unordered-collection/positive")),
        Ok(()),
        "unordered-collection/positive"
    );
}

#[test]
fn unordered_collection_negative() {
    assert_eq!(
        check_case(&Name::of("unordered-collection/negative")),
        Ok(()),
        "unordered-collection/negative"
    );
}

#[test]
fn float_type_positive() {
    assert_eq!(
        check_case(&Name::of("float-type/positive")),
        Ok(()),
        "float-type/positive"
    );
}

#[test]
fn float_type_negative() {
    assert_eq!(
        check_case(&Name::of("float-type/negative")),
        Ok(()),
        "float-type/negative"
    );
}

#[test]
fn ambient_input_positive() {
    assert_eq!(
        check_case(&Name::of("ambient-input/positive")),
        Ok(()),
        "ambient-input/positive"
    );
}

#[test]
fn ambient_input_shell_allowed() {
    assert_eq!(
        check_case(&Name::of("ambient-input/shell-allowed")),
        Ok(()),
        "ambient-input/shell-allowed"
    );
}

#[test]
fn ambient_input_shell_other_path() {
    assert_eq!(
        check_case(&Name::of("ambient-input/shell-other-path")),
        Ok(()),
        "ambient-input/shell-other-path"
    );
}

#[test]
fn ambient_input_negative() {
    assert_eq!(
        check_case(&Name::of("ambient-input/negative")),
        Ok(()),
        "ambient-input/negative"
    );
}

#[test]
fn numeric_cast_positive() {
    assert_eq!(
        check_case(&Name::of("numeric-cast/positive")),
        Ok(()),
        "numeric-cast/positive"
    );
}

#[test]
fn numeric_cast_negative() {
    assert_eq!(
        check_case(&Name::of("numeric-cast/negative")),
        Ok(()),
        "numeric-cast/negative"
    );
}

#[test]
fn cfg_clippy_positive() {
    assert_eq!(
        check_case(&Name::of("cfg-clippy/positive")),
        Ok(()),
        "cfg-clippy/positive"
    );
}

#[test]
fn cfg_clippy_negative() {
    assert_eq!(
        check_case(&Name::of("cfg-clippy/negative")),
        Ok(()),
        "cfg-clippy/negative"
    );
}

#[test]
fn catch_all_arm_wildcard() {
    assert_eq!(
        check_case(&Name::of("catch-all-arm/wildcard")),
        Ok(()),
        "catch-all-arm/wildcard"
    );
}

#[test]
fn catch_all_arm_binding() {
    assert_eq!(
        check_case(&Name::of("catch-all-arm/binding")),
        Ok(()),
        "catch-all-arm/binding"
    );
}

#[test]
fn catch_all_arm_tuple_exhaustive() {
    assert_eq!(
        check_case(&Name::of("catch-all-arm/tuple-exhaustive")),
        Ok(()),
        "catch-all-arm/tuple-exhaustive"
    );
}

#[test]
fn catch_all_arm_string_scrutinee() {
    assert_eq!(
        check_case(&Name::of("catch-all-arm/string-scrutinee")),
        Ok(()),
        "catch-all-arm/string-scrutinee"
    );
}

#[test]
fn catch_all_arm_glob_import() {
    assert_eq!(
        check_case(&Name::of("catch-all-arm/glob-import")),
        Ok(()),
        "catch-all-arm/glob-import"
    );
}

#[test]
fn catch_all_arm_named_variants() {
    assert_eq!(
        check_case(&Name::of("catch-all-arm/named-variants")),
        Ok(()),
        "catch-all-arm/named-variants"
    );
}

#[test]
fn catch_all_arm_std_enum() {
    assert_eq!(
        check_case(&Name::of("catch-all-arm/std-enum")),
        Ok(()),
        "catch-all-arm/std-enum"
    );
}

#[test]
fn refutable_shortcut_if_let() {
    assert_eq!(
        check_case(&Name::of("refutable-shortcut/if-let")),
        Ok(()),
        "refutable-shortcut/if-let"
    );
}

#[test]
fn refutable_shortcut_other_forms() {
    assert_eq!(
        check_case(&Name::of("refutable-shortcut/other-forms")),
        Ok(()),
        "refutable-shortcut/other-forms"
    );
}

#[test]
fn refutable_shortcut_negative() {
    assert_eq!(
        check_case(&Name::of("refutable-shortcut/negative")),
        Ok(()),
        "refutable-shortcut/negative"
    );
}

#[test]
fn banned_aliased_module() {
    assert_eq!(
        check_case(&Name::of("banned/aliased-module")),
        Ok(()),
        "banned/aliased-module"
    );
}

#[test]
fn banned_direct_and_macro() {
    assert_eq!(
        check_case(&Name::of("banned/direct-and-macro")),
        Ok(()),
        "banned/direct-and-macro"
    );
}

#[test]
fn banned_outside_files_glob() {
    assert_eq!(
        check_case(&Name::of("banned/outside-files-glob")),
        Ok(()),
        "banned/outside-files-glob"
    );
}

#[test]
fn banned_allowed_path() {
    assert_eq!(
        check_case(&Name::of("banned/allowed-path")),
        Ok(()),
        "banned/allowed-path"
    );
}

#[test]
fn banned_other_dimensions() {
    assert_eq!(
        check_case(&Name::of("banned/other-dimensions")),
        Ok(()),
        "banned/other-dimensions"
    );
}

#[test]
fn banned_other_dimensions_negative() {
    assert_eq!(
        check_case(&Name::of("banned/other-dimensions-negative")),
        Ok(()),
        "banned/other-dimensions-negative"
    );
}

#[test]
fn case_test_assertion_no_assertion() {
    assert_eq!(
        check_case(&Name::of("test-assertion/no-assertion")),
        Ok(()),
        "test-assertion/no-assertion"
    );
}

#[test]
fn case_test_assertion_result_only() {
    assert_eq!(
        check_case(&Name::of("test-assertion/result-only")),
        Ok(()),
        "test-assertion/result-only"
    );
}

#[test]
fn case_test_assertion_negative() {
    assert_eq!(
        check_case(&Name::of("test-assertion/negative")),
        Ok(()),
        "test-assertion/negative"
    );
}

#[test]
fn case_test_assertion_custom_assertion_macro() {
    assert_eq!(
        check_case(&Name::of("test-assertion/custom-assertion-macro")),
        Ok(()),
        "test-assertion/custom-assertion-macro"
    );
}

#[test]
fn case_test_early_return_positive() {
    assert_eq!(
        check_case(&Name::of("test-early-return/positive")),
        Ok(()),
        "test-early-return/positive"
    );
}

#[test]
fn case_test_early_return_negative() {
    assert_eq!(
        check_case(&Name::of("test-early-return/negative")),
        Ok(()),
        "test-early-return/negative"
    );
}

#[test]
fn case_test_early_return_configured_attribute() {
    assert_eq!(
        check_case(&Name::of("test-early-return/configured-attribute")),
        Ok(()),
        "test-early-return/configured-attribute"
    );
}

#[test]
fn macro_needs_comment_positive() {
    assert_eq!(
        check_case(&Name::of("macro-needs-comment/positive")),
        Ok(()),
        "macro-needs-comment/positive"
    );
}

#[test]
fn macro_needs_comment_negative() {
    assert_eq!(
        check_case(&Name::of("macro-needs-comment/negative")),
        Ok(()),
        "macro-needs-comment/negative"
    );
}

#[test]
fn macro_needs_comment_too_far() {
    assert_eq!(
        check_case(&Name::of("macro-needs-comment/too-far")),
        Ok(()),
        "macro-needs-comment/too-far"
    );
}

#[test]
fn case_test_attributes_positive() {
    assert_eq!(
        check_case(&Name::of("test-attributes/positive")),
        Ok(()),
        "test-attributes/positive"
    );
}

#[test]
fn case_test_attributes_negative() {
    assert_eq!(
        check_case(&Name::of("test-attributes/negative")),
        Ok(()),
        "test-attributes/negative"
    );
}

#[test]
fn case_test_attributes_configured_attribute() {
    assert_eq!(
        check_case(&Name::of("test-attributes/configured-attribute")),
        Ok(()),
        "test-attributes/configured-attribute"
    );
}

#[test]
fn unparsed_macro_positive() {
    assert_eq!(
        check_case(&Name::of("unparsed-macro/positive")),
        Ok(()),
        "unparsed-macro/positive"
    );
}

#[test]
fn unparsed_macro_negative() {
    assert_eq!(
        check_case(&Name::of("unparsed-macro/negative")),
        Ok(()),
        "unparsed-macro/negative"
    );
}

#[test]
fn struct_rest_pattern_positive() {
    assert_eq!(
        check_case(&Name::of("struct-rest-pattern/positive")),
        Ok(()),
        "struct-rest-pattern/positive"
    );
}

#[test]
fn struct_rest_pattern_negative() {
    assert_eq!(
        check_case(&Name::of("struct-rest-pattern/negative")),
        Ok(()),
        "struct-rest-pattern/negative"
    );
}

#[test]
fn struct_rest_pattern_resolution() {
    assert_eq!(
        check_case(&Name::of("struct-rest-pattern/resolution")),
        Ok(()),
        "struct-rest-pattern/resolution"
    );
}

#[test]
fn struct_rest_pattern_catch_all_overlap() {
    assert_eq!(
        check_case(&Name::of("struct-rest-pattern/catch-all-overlap")),
        Ok(()),
        "struct-rest-pattern/catch-all-overlap"
    );
}

#[test]
fn struct_rest_pattern_refutable_overlap() {
    assert_eq!(
        check_case(&Name::of("struct-rest-pattern/refutable-overlap")),
        Ok(()),
        "struct-rest-pattern/refutable-overlap"
    );
}

#[test]
fn struct_rest_pattern_macro_bodies() {
    assert_eq!(
        check_case(&Name::of("struct-rest-pattern/macro-bodies")),
        Ok(()),
        "struct-rest-pattern/macro-bodies"
    );
}

#[test]
fn memory_leak_positive() {
    assert_eq!(
        check_case(&Name::of("memory-leak/positive")),
        Ok(()),
        "memory-leak/positive"
    );
}

#[test]
fn memory_leak_negative() {
    assert_eq!(
        check_case(&Name::of("memory-leak/negative")),
        Ok(()),
        "memory-leak/negative"
    );
}

#[test]
fn memory_leak_resolution() {
    assert_eq!(
        check_case(&Name::of("memory-leak/resolution")),
        Ok(()),
        "memory-leak/resolution"
    );
}

#[test]
fn struct_rest_pattern_fieldless() {
    assert_eq!(
        check_case(&Name::of("struct-rest-pattern/fieldless")),
        Ok(()),
        "struct-rest-pattern/fieldless"
    );
}

#[test]
fn default_construction_positive() {
    assert_eq!(
        check_case(&Name::of("default-construction/positive")),
        Ok(()),
        "default-construction/positive"
    );
}

#[test]
fn default_construction_negative() {
    assert_eq!(
        check_case(&Name::of("default-construction/negative")),
        Ok(()),
        "default-construction/negative"
    );
}

#[test]
fn default_construction_resolution() {
    assert_eq!(
        check_case(&Name::of("default-construction/resolution")),
        Ok(()),
        "default-construction/resolution"
    );
}

#[test]
fn default_construction_macro_bodies() {
    assert_eq!(
        check_case(&Name::of("default-construction/macro-bodies")),
        Ok(()),
        "default-construction/macro-bodies"
    );
}

#[test]
fn unused_underscore_binding_positive() {
    assert_eq!(
        check_case(&Name::of("unused-underscore-binding/positive")),
        Ok(()),
        "unused-underscore-binding/positive"
    );
}

#[test]
fn unused_underscore_binding_negative() {
    assert_eq!(
        check_case(&Name::of("unused-underscore-binding/negative")),
        Ok(()),
        "unused-underscore-binding/negative"
    );
}

#[test]
fn unused_underscore_binding_macro_bodies() {
    assert_eq!(
        check_case(&Name::of("unused-underscore-binding/macro-bodies")),
        Ok(()),
        "unused-underscore-binding/macro-bodies"
    );
}

#[test]
fn dropped_call_result_positive() {
    assert_eq!(
        check_case(&Name::of("dropped-call-result/positive")),
        Ok(()),
        "dropped-call-result/positive"
    );
}

#[test]
fn dropped_call_result_negative() {
    assert_eq!(
        check_case(&Name::of("dropped-call-result/negative")),
        Ok(()),
        "dropped-call-result/negative"
    );
}

#[test]
fn dropped_call_result_resolution() {
    assert_eq!(
        check_case(&Name::of("dropped-call-result/resolution")),
        Ok(()),
        "dropped-call-result/resolution"
    );
}

#[test]
fn dropped_call_result_macro_bodies() {
    assert_eq!(
        check_case(&Name::of("dropped-call-result/macro-bodies")),
        Ok(()),
        "dropped-call-result/macro-bodies"
    );
}

#[test]
fn swallowed_error_positive() {
    assert_eq!(
        check_case(&Name::of("swallowed-error/positive")),
        Ok(()),
        "swallowed-error/positive"
    );
}

#[test]
fn swallowed_error_negative() {
    assert_eq!(
        check_case(&Name::of("swallowed-error/negative")),
        Ok(()),
        "swallowed-error/negative"
    );
}

#[test]
fn swallowed_error_overlap() {
    assert_eq!(
        check_case(&Name::of("swallowed-error/overlap")),
        Ok(()),
        "swallowed-error/overlap"
    );
}

#[test]
fn swallowed_error_let_chain() {
    assert_eq!(
        check_case(&Name::of("swallowed-error/let-chain")),
        Ok(()),
        "swallowed-error/let-chain"
    );
}

#[test]
fn swallowed_error_resolution() {
    assert_eq!(
        check_case(&Name::of("swallowed-error/resolution")),
        Ok(()),
        "swallowed-error/resolution"
    );
}

#[test]
fn banned_core_config_std_code() {
    assert_eq!(
        check_case(&Name::of("banned/core-config-std-code")),
        Ok(()),
        "banned/core-config-std-code"
    );
}

#[test]
fn banned_std_config_core_code() {
    assert_eq!(
        check_case(&Name::of("banned/std-config-core-code")),
        Ok(()),
        "banned/std-config-core-code"
    );
}

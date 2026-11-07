//! Utility functions for identifier and string manipulation
//!
//! This module provides helper functions for converting strings to valid Rust
//! identifiers, handling edge cases like empty strings, numeric prefixes,
//! and invalid characters.

use std::collections::HashSet;
use stringcase::camel_case;

/// A helper function to convert a string to a valid Rust identifier in UpperCamelCase.
///
/// This function sanitizes input to ensure the result is always a valid Rust identifier:
/// - Empty strings become "EmptyValue"
/// - Strings starting with numbers get prefixed with "Value"
/// - Invalid characters are removed or replaced
/// - Pure whitespace becomes "EmptyValue"
///
/// Distinct source strings can lower to the same result when they differ only by
/// non-alphanumeric characters (e.g. `YYYY-MM-DD` and `YYYY/MM/DD` both become
/// `YyyyMmDd`). Callers that need unique idents per source must use
/// [`allocate_unique_ident`] after sanitization.
pub fn to_upper_camel_case(s: &str) -> String {
    // Handle empty or whitespace-only strings
    if s.trim().is_empty() {
        return "EmptyValue".to_string();
    }

    // Sanitize the string: keep only alphanumeric chars and convert to appropriate case
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();

    // Convert to UpperCamelCase
    let converted = {
        let camel = camel_case(&cleaned);
        if let Some(first_char) = camel.chars().next() {
            first_char.to_uppercase().to_string() + &camel[1..]
        } else {
            "EmptyValue".to_string()
        }
    };

    // Ensure the result starts with a letter or underscore (valid Rust identifier)
    let result = if let Some(first) = converted.chars().next() {
        if first.is_alphabetic() || first == '_' {
            converted
        } else {
            // Starts with invalid char (like number), prefix with "Value"
            format!("Value{}", converted)
        }
    } else {
        // Empty after processing, use fallback
        "EmptyValue".to_string()
    };

    // Final validation: ensure result is non-empty and starts correctly
    if result.is_empty() {
        "EmptyValue".to_string()
    } else {
        result
    }
}

/// Names that generated code uses unqualified at the crate root: the Rust prelude
/// types, traits, and variant constructors, `Self`, and the helper types that the
/// generator defines at the crate root itself. A schema type with one of these names
/// would shadow the real item for the whole generated crate (`pub struct String`
/// breaks every `String` field, `pub struct Default` breaks `impl Default`).
const RESERVED_TYPE_NAMES: &[&str] = &[
    "Self",
    "Option",
    "Some",
    "None",
    "Result",
    "Ok",
    "Err",
    "Vec",
    "String",
    "Box",
    "Copy",
    "Send",
    "Sized",
    "Sync",
    "Unpin",
    "Drop",
    "Fn",
    "FnMut",
    "FnOnce",
    "AsMut",
    "AsRef",
    "From",
    "Into",
    "DoubleEndedIterator",
    "ExactSizeIterator",
    "Extend",
    "IntoIterator",
    "Iterator",
    "Clone",
    "Default",
    "Eq",
    "Ord",
    "PartialEq",
    "PartialOrd",
    "ToOwned",
    "ToString",
    "TryFrom",
    "TryInto",
    "FromIterator",
    "GapInfo",
    "PartialEncoding",
];

/// True when a generated type with this name would shadow an item that generated
/// code relies on (see [`to_type_name`]).
pub fn is_reserved_type_name(name: &str) -> bool {
    RESERVED_TYPE_NAMES.contains(&name)
}

/// Name of the generated Rust type for a schema definition: [`to_upper_camel_case`],
/// with a `Type` suffix when the result would shadow a Rust prelude item
/// (`string` becomes `StringType`, `default` becomes `DefaultType`).
///
/// Use this for type names only. Enum variant names keep [`to_upper_camel_case`]
/// because variants live in their enum's namespace and cannot shadow anything.
pub fn to_type_name(s: &str) -> String {
    let name = to_upper_camel_case(s);
    if is_reserved_type_name(&name) {
        format!("{name}Type")
    } else {
        name
    }
}

/// Allocate a unique Rust identifier from a preferred base name.
///
/// The first claim of `base` returns `base` unchanged. Further claims append a
/// decimal suffix starting at `2` (`Base`, `Base2`, `Base3`, …) so PascalCase
/// idents stay valid for `format_ident!` and match arms.
///
/// Allocation is deterministic for a given claim order. Used for enum string
/// variants (and available for type naming) when
/// [`to_upper_camel_case`] collapses distinct wire strings.
pub fn allocate_unique_ident(base: &str, used: &mut HashSet<String>) -> String {
    let preferred = if base.is_empty() {
        "EmptyValue".to_string()
    } else {
        base.to_string()
    };

    if used.insert(preferred.clone()) {
        return preferred;
    }

    let mut suffix = 2u32;
    loop {
        let candidate = format!("{preferred}{suffix}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        suffix = suffix.saturating_add(1);
        if suffix == u32::MAX {
            // Extremely unlikely; fall back to a unique sentinel.
            let fallback = format!("{preferred}Fallback");
            used.insert(fallback.clone());
            return fallback;
        }
    }
}

/// Sanitize `source` to UpperCamelCase and allocate a unique ident in `used`.
pub fn allocate_unique_upper_camel_ident(source: &str, used: &mut HashSet<String>) -> String {
    allocate_unique_ident(&to_upper_camel_case(source), used)
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_type_name_suffixes_names_that_shadow_prelude_items() {
        for (schema_name, expected) in [
            ("string", "StringType"),
            ("String", "StringType"),
            ("default", "DefaultType"),
            ("result", "ResultType"),
            ("option", "OptionType"),
            ("vec", "VecType"),
            ("box", "BoxType"),
            ("self", "SelfType"),
            ("none", "NoneType"),
            ("clone", "CloneType"),
            ("partial_encoding", "PartialEncodingType"),
            ("gap-info", "GapInfoType"),
        ] {
            assert_eq!(to_type_name(schema_name), expected, "{schema_name}");
        }
    }

    #[test]
    fn test_to_type_name_leaves_other_names_alone() {
        for (schema_name, expected) in [
            ("hello_world", "HelloWorld"),
            ("strings", "Strings"),
            ("string_array", "StringArray"),
            ("StringType", "StringType"),
            ("my_result", "MyResult"),
            ("123invalid", "Value123Invalid"),
            ("", "EmptyValue"),
        ] {
            assert_eq!(to_type_name(schema_name), expected, "{schema_name}");
        }
    }

    #[test]
    fn test_to_type_name_is_idempotent() {
        // Callers re-sanitize names that discovery has already assigned.
        for schema_name in ["string", "default", "Self", "user_profile", "result"] {
            let once = to_type_name(schema_name);
            assert_eq!(to_type_name(&once), once, "{schema_name}");
        }
    }

    #[test]
    fn test_variant_names_keep_using_the_plain_camel_case() {
        // Variants live in their enum's namespace and cannot shadow anything, so
        // renaming them would only change the API of schemas that work today.
        assert_eq!(to_upper_camel_case("string"), "String");
        assert_eq!(to_upper_camel_case("default"), "Default");
    }

    #[test]
    fn test_to_upper_camel_case() {
        // Normal cases
        assert_eq!(to_upper_camel_case("hello_world"), "HelloWorld");
        assert_eq!(to_upper_camel_case("alreadyCamel"), "AlreadyCamel");
        assert_eq!(to_upper_camel_case("kebab-case-string"), "KebabCaseString");
        assert_eq!(to_upper_camel_case("a"), "A");

        // Robust edge cases that should not crash
        assert_eq!(to_upper_camel_case(""), "EmptyValue");
        assert_eq!(to_upper_camel_case("   "), "EmptyValue"); // Whitespace only
        assert_eq!(to_upper_camel_case("\0"), "EmptyValue"); // Null char
        assert_eq!(to_upper_camel_case("123invalid"), "Value123Invalid"); // Starts with number
        assert_eq!(to_upper_camel_case("special@chars"), "SpecialChars"); // Special chars
        assert_eq!(to_upper_camel_case("🦀🔥"), "EmptyValue"); // Unicode -> fallback
    }

    #[test]
    fn punctuation_only_differences_collapse_to_same_camel_case() {
        // Documents the sanitizer collision class fixed by allocate_unique_ident
        // for enum string variants (BUG-1 / BUG-4).
        assert_eq!(
            to_upper_camel_case("YYYY-MM-DD"),
            to_upper_camel_case("YYYY/MM/DD")
        );
        assert_eq!(to_upper_camel_case("YYYY-MM-DD"), "YyyyMmDd");
        assert_eq!(
            to_upper_camel_case("foo-bar"),
            to_upper_camel_case("foo_bar")
        );
        assert_eq!(
            to_upper_camel_case("foo-bar"),
            to_upper_camel_case("foo.bar")
        );
        assert_eq!(
            to_upper_camel_case("HTTP/1.1"),
            to_upper_camel_case("HTTP-1.1")
        );
    }

    #[test]
    fn allocate_unique_ident_disambiguates_collisions() {
        let mut used = HashSet::new();
        assert_eq!(allocate_unique_ident("YyyyMmDd", &mut used), "YyyyMmDd");
        assert_eq!(allocate_unique_ident("YyyyMmDd", &mut used), "YyyyMmDd2");
        assert_eq!(allocate_unique_ident("YyyyMmDd", &mut used), "YyyyMmDd3");
        assert_eq!(allocate_unique_ident("Other", &mut used), "Other");
    }

    #[test]
    fn allocate_unique_upper_camel_ident_for_date_format_enums() {
        let mut used = HashSet::new();
        let first = allocate_unique_upper_camel_ident("YYYY-MM-DD", &mut used);
        let second = allocate_unique_upper_camel_ident("YYYY/MM/DD", &mut used);
        assert_eq!(first, "YyyyMmDd");
        assert_eq!(second, "YyyyMmDd2");
        assert_ne!(first, second);
    }

    #[test]
    fn test_identifier_safety() {
        let dangerous_inputs = vec![
            "",
            "\0",
            "   ",
            "\n\t\r",
            "123",
            "123invalid",
            "special@#$%chars",
            "🦀🔥",
            "very-long-kebab-case-with-many-special-chars!@#$%^&*()",
        ];

        for input in dangerous_inputs {
            let result = to_upper_camel_case(input);

            // Must not be empty
            assert!(
                !result.is_empty(),
                "Input '{}' produced empty result",
                input
            );

            // Must start with letter or underscore
            if let Some(first) = result.chars().next() {
                assert!(
                    first.is_alphabetic() || first == '_',
                    "Input '{}' produced invalid identifier '{}' (starts with '{}')",
                    input,
                    result,
                    first
                );
            }

            println!("✅ '{}' -> '{}' (valid identifier)", input, result);
        }
    }
}

use unic_langid::LanguageIdentifier;

use crate::prelude::*;

// Strip isolation chars for testing (Bidi control characters added by Fluent)
fn clean(s: &str) -> String {
    s.replace(['\u{2068}', '\u{2069}'], "")
}

fn create_map(locale: LanguageIdentifier, fluent_source: &str) -> TranslationMap {
    let file = TranslationFile::from_str(vec![locale.clone()], fluent_source).expect("Failed to parse fluent source");
    let map = TranslationMap::new(locale);
    map.add_translation(file)
}

#[test]
fn basic_resolution() {
    let source = "hello = Hello World";
    let map = create_map(langid!("en-US"), source);

    let s = LocalizedStringBuilder::new("hello").build();
    assert_eq!(&*s.resolve(&map), "Hello World");
}

#[test]
fn missing_key_fallback() {
    let source = "hello = Hello World";
    let map = create_map(langid!("en-US"), source);

    // Uses placeholder if provided
    let s = LocalizedStringBuilder::new("missing_key").placeholder("Fallback").build();
    assert_eq!(&*s.resolve(&map), "Fallback");

    // Uses key if placeholder missing
    let s2 = LocalizedStringBuilder::new("missing_key").build();
    assert_eq!(&*s2.resolve(&map), "missing_key");
}

#[test]
fn arguments_string() {
    let source = "welcome = Welcome, { $name }!";
    let map = create_map(langid!("en-US"), source);

    let s = LocalizedStringBuilder::new("welcome").arg("name", "Alice").build();

    assert_eq!(clean(&s.resolve(&map)), "Welcome, Alice!");
}

#[test]
fn arguments_number_basic() {
    let source = "items = You have { $count } items.";
    let map = create_map(langid!("en-US"), source);

    let s = LocalizedStringBuilder::new("items").arg("count", 5.0).build();

    assert_eq!(clean(&s.resolve(&map)), "You have 5 items.");
}

#[test]
fn fmt_minimum_integer_digits() {
    let source = "num = { NUMBER($val, minimumIntegerDigits: 3) }";
    let map = create_map(langid!("en-US"), source);

    // Simple padding
    let s1 = LocalizedStringBuilder::new("num").arg("val", 5.0).build();
    assert_eq!(clean(&s1.resolve(&map)), "005");

    // No padding needed
    let s2 = LocalizedStringBuilder::new("num").arg("val", 123.0).build();
    assert_eq!(clean(&s2.resolve(&map)), "123");

    // Negative padding
    let s3 = LocalizedStringBuilder::new("num").arg("val", -5.0).build();
    assert_eq!(clean(&s3.resolve(&map)), "-005");

    // Existing decimals shouldn't affect integer padding
    let s4 = LocalizedStringBuilder::new("num").arg("val", 5.12).build();
    assert_eq!(clean(&s4.resolve(&map)), "005.12"); // ICU often defaults to maxFrac: 3
}

#[test]
fn fmt_fraction_digits() {
    let source = r#"
min = { NUMBER($val, minimumFractionDigits: 2) }
max = { NUMBER($val, maximumFractionDigits: 2) }
mixed = { NUMBER($val, minimumFractionDigits: 2, maximumFractionDigits: 4) }
    "#;
    let map = create_map(langid!("en-US"), source);

    // Minimum: Pad zeros
    let s1 = LocalizedStringBuilder::new("min").arg("val", 1.0).build();
    assert_eq!(clean(&s1.resolve(&map)), "1.00");

    let s2 = LocalizedStringBuilder::new("min").arg("val", 1.1).build();
    assert_eq!(clean(&s2.resolve(&map)), "1.10");

    // Maximum: Rounding/Truncation
    let s3 = LocalizedStringBuilder::new("max").arg("val", 1.23456).build();
    assert_eq!(clean(&s3.resolve(&map)), "1.23");

    let s4 = LocalizedStringBuilder::new("max").arg("val", 1.239).build();
    assert_eq!(clean(&s4.resolve(&map)), "1.24");

    // Explicit max=2 on integer:
    let s5 = LocalizedStringBuilder::new("max").arg("val", 100.0).build();
    assert_eq!(clean(&s5.resolve(&map)), "100");

    // Mixed: Pad to 2, keep up to 4
    let s6 = LocalizedStringBuilder::new("mixed").arg("val", 1.1).build();
    assert_eq!(clean(&s6.resolve(&map)), "1.10");

    let s7 = LocalizedStringBuilder::new("mixed").arg("val", 1.123456).build();
    assert_eq!(clean(&s7.resolve(&map)), "1.1235");
}

#[test]
fn fmt_grouping() {
    let source = r#"
on = { NUMBER($val, useGrouping: "true") }
off = { NUMBER($val, useGrouping: "false") }
default = { NUMBER($val) }
    "#;
    let map = create_map(langid!("en-US"), source);

    // Explicit On
    let s1 = LocalizedStringBuilder::new("on").arg("val", 10000.0).build();
    assert_eq!(clean(&s1.resolve(&map)), "10,000");

    // Explicit Off
    let s2 = LocalizedStringBuilder::new("off").arg("val", 10000.0).build();
    assert_eq!(clean(&s2.resolve(&map)), "10000");

    // Large numbers
    let s3 = LocalizedStringBuilder::new("on").arg("val", 1000000.0).build();
    assert_eq!(clean(&s3.resolve(&map)), "1,000,000");

    // Negative numbers
    let s4 = LocalizedStringBuilder::new("on").arg("val", -1234.5).build();
    assert_eq!(clean(&s4.resolve(&map)), "-1,234.5");
}

#[test]
fn fmt_significant_digits_max() {
    // ECMA-402: Significant digits override fraction digits.
    let source = "num = { NUMBER($val, maximumSignificantDigits: 3) }";
    let map = create_map(langid!("en-US"), source);

    // 12345 -> 12300 (Round to 3 significant figures)
    let s1 = LocalizedStringBuilder::new("num").arg("val", 12345.0).build();
    assert_eq!(clean(&s1.resolve(&map)), "12,300");

    // 0.12345 -> 0.123
    let s2 = LocalizedStringBuilder::new("num").arg("val", 0.12345).build();
    assert_eq!(clean(&s2.resolve(&map)), "0.123");

    // 1.2345 -> 1.23
    let s3 = LocalizedStringBuilder::new("num").arg("val", 1.2345).build();
    assert_eq!(clean(&s3.resolve(&map)), "1.23");

    // 1.0001 -> 1 (Since max sig is 3)
    let s4 = LocalizedStringBuilder::new("num").arg("val", 1.0001).build();
    assert_eq!(clean(&s4.resolve(&map)), "1");
}

#[test]
fn fmt_significant_digits_min() {
    let source = "num = { NUMBER($val, minimumSignificantDigits: 3) }";
    let map = create_map(langid!("en-US"), source);

    // 5 -> 5.00
    let s1 = LocalizedStringBuilder::new("num").arg("val", 5.0).build();
    assert_eq!(clean(&s1.resolve(&map)), "5.00");

    // 0.5 -> 0.500
    let s2 = LocalizedStringBuilder::new("num").arg("val", 0.5).build();
    assert_eq!(clean(&s2.resolve(&map)), "0.500");

    // 1234 -> 1234 (Met requirement)
    let s3 = LocalizedStringBuilder::new("num").arg("val", 1234.0).build();
    assert_eq!(clean(&s3.resolve(&map)), "1,234");
}

#[test]
fn fmt_zero() {
    // Zero is a special case for log10 calculations often used in manual formatting
    let source = r#"
basic = { NUMBER($val) }
sig   = { NUMBER($val, minimumSignificantDigits: 2) }
    "#;
    let map = create_map(langid!("en-US"), source);

    let s1 = LocalizedStringBuilder::new("basic").arg("val", 0.0).build();
    assert_eq!(clean(&s1.resolve(&map)), "0");

    // 0 -> 0.0 (2 significant digits for zero usually implies decimals)
    let s2 = LocalizedStringBuilder::new("sig").arg("val", 0.0).build();
    assert_eq!(clean(&s2.resolve(&map)), "0.0");
}

#[test]
fn fmt_min_fraction_with_zero_max_triggers_dot_push() {
    let source = r#"num = { NUMBER($val, minimumFractionDigits: 2, maximumFractionDigits: 0) }"#;
    let map = create_map(langid!("en-US"), source);

    let s = LocalizedStringBuilder::new("num").arg("val", 1.0).build();
    assert_eq!(clean(&s.resolve(&map)), "1");
}

#[test]
fn test_dead_dependency_invalidates_cache() {
    let lang = langid!("en-US");
    let ftl_string = "greeting = Hello, { $name }!";
    let file = TranslationFile::from_str(vec![lang.clone()], ftl_string).expect("Failed to create translation file");

    let map = TranslationMap::new(lang).add_translation(file);

    let name_ref = {
        let name_var = Var::new("Alice".to_string());
        let name_ref = name_var.downgrade();

        let loc_string = LocalizedStringBuilder::new("greeting").arg("name", name_ref).build();

        {
            let result = loc_string.resolve(&map);
            assert_eq!(clean(&result), "Hello, Alice!");
        }

        // Drop name_var here at the end of the scope.
        // The Var is now "dead", but loc_string holds a WeakVar to it.
        // name_ref is returned so we can use it in step 6.
        (loc_string, name_ref)
    };

    let (loc_string, name_ref) = name_ref;

    assert!(!name_ref.is_alive(), "Variable should be dead for this test to be valid");

    // The resolve logic should notice that a previously tracked dependency returned None,
    // invalidate the cache, and re-run fluent, falling back to the key.
    let result = loc_string.resolve(&map);

    assert_eq!(&*result, "greeting", "Cache was not invalidated after dependency died.");
}

#[test]
fn fmt_minimum_fraction_digits_should_not_round_without_maximum() {
    // minimumFractionDigits should ONLY pad, not reduce precision.
    let source = r#"min = { NUMBER($val, minimumFractionDigits: 2) }"#;
    let map = create_map(langid!("en-US"), source);

    let s = LocalizedStringBuilder::new("min").arg("val", 1.23456).build();
    assert_eq!(clean(&s.resolve(&map)), "1.23456");
}

#[test]
fn fmt_number_nan_and_infinity_should_passthrough() {
    // Expected behavior: passthrough string form
    let source = r#"n = { NUMBER($val) }"#;
    let map = create_map(langid!("en-US"), source);

    let nan = LocalizedStringBuilder::new("n").arg("val", f64::NAN).build();
    assert_eq!(clean(&nan.resolve(&map)), "NaN");

    let pos_inf = LocalizedStringBuilder::new("n").arg("val", f64::INFINITY).build();
    assert_eq!(clean(&pos_inf.resolve(&map)), "inf");

    let neg_inf = LocalizedStringBuilder::new("n").arg("val", f64::NEG_INFINITY).build();
    assert_eq!(clean(&neg_inf.resolve(&map)), "-inf");
}

#[test]
fn fmt_significant_digits_max_should_not_collapse_tiny_numbers_to_zero() {
    let source = r#"num = { NUMBER($val, maximumSignificantDigits: 21) }"#;
    let map = create_map(langid!("en-US"), source);

    let tiny = f64::MIN_POSITIVE; // ~2.225e-308
    let s = LocalizedStringBuilder::new("num").arg("val", tiny).build();
    let out = clean(&s.resolve(&map));

    assert_ne!(out, "0", "tiny number formatted as zero: {out}");
    assert_ne!(out, "NaN", "tiny number formatted as NaN: {out}");

    // en-US output should be parseable as f64 (either fixed or scientific),
    // and it must not parse back to zero.
    let parsed: f64 = out.parse().expect("output should parse as f64 in en-US");
    assert!(parsed > 0.0, "formatted tiny number should be > 0, got {parsed} from {out}");
}

#[cfg(feature = "icu")]
mod icu_features {
    use super::*;
    use time::{Date, Month, Time};

    #[test]
    fn locale_awareness() {
        // German uses commas for decimals and dots for grouping
        let source = "num = { NUMBER($val) }";
        let map = create_map(langid!("de-DE"), source);

        let s = LocalizedStringBuilder::new("num").arg("val", 1234.56).build();
        assert_eq!(clean(&s.resolve(&map)), "1.234,56");
    }

    #[test]
    fn date_formatting_basic() {
        // Test internal DATETIME function exposed by localization.rs
        let source = r#"
default = { DATETIME($d) }
short = { DATETIME($d, dateStyle: "short") }
medium = { DATETIME($d, dateStyle: "medium") }
        "#;
        let map = create_map(langid!("en-US"), source);

        // Construct 2024-01-20 15:30:00 using time crate
        let date = Date::from_calendar_date(2024, Month::January, 20).expect("Failed to create date");
        let time = Time::from_hms(15, 30, 0).expect("Failed to create time");
        let dt = date.with_time(time).assume_utc();

        // Pass via Arg
        let s1 = LocalizedStringBuilder::new("default").arg("d", dt).build();
        // Default usually corresponds to medium date
        assert_eq!(clean(&s1.resolve(&map)), "Jan 20, 2024");

        let s2 = LocalizedStringBuilder::new("short").arg("d", dt).build();
        assert_eq!(clean(&s2.resolve(&map)), "1/20/24");

        let s3 = LocalizedStringBuilder::new("medium").arg("d", dt).build();
        assert_eq!(clean(&s3.resolve(&map)), "Jan 20, 2024");
    }

    #[test]
    fn date_formatting_string_input() {
        // The implementation allows string inputs to DATETIME if they parse as ISO
        let source = r#"dt = { DATETIME($d, dateStyle: "short") }"#;
        let map = create_map(langid!("en-US"), source);

        let s = LocalizedStringBuilder::new("dt").arg("d", "2024-01-20T15:30:00").build();
        assert_eq!(clean(&s.resolve(&map)), "1/20/24");
    }

    #[test]
    fn date_formatting_string_input_rfc3339_zulu() {
        // Covers parsing of RFC3339 timestamps that include 'Z' (UTC).
        let source = r#"dt = { DATETIME($d, dateStyle: "short") }"#;
        let map = create_map(langid!("en-US"), source);

        let s = LocalizedStringBuilder::new("dt").arg("d", "2024-01-20T15:30:00Z").build();
        assert_eq!(clean(&s.resolve(&map)), "1/20/24");
    }

    #[test]
    fn date_formatting_string_input_rfc3339_with_offset() {
        // Covers parsing of RFC3339 timestamps with an explicit numeric offset.
        let source = r#"dt = { DATETIME($d, dateStyle: "short") }"#;
        let map = create_map(langid!("en-US"), source);

        let s = LocalizedStringBuilder::new("dt").arg("d", "2024-01-20T15:30:00-07:00").build();
        assert_eq!(clean(&s.resolve(&map)), "1/20/24");
    }

    #[test]
    fn date_time_formatting_combined_styles_seconds_branching() {
        let source = r#"
short_short = { DATETIME($d, dateStyle: "short", timeStyle: "short") }
short_medium = { DATETIME($d, dateStyle: "short", timeStyle: "medium") }
        "#;
        let map = create_map(langid!("en-US"), source);

        let date = Date::from_calendar_date(2024, Month::January, 20).expect("Failed to create date");
        let time = Time::from_hms(15, 30, 0).expect("Failed to create time");
        let dt = date.with_time(time).assume_utc();

        let s1 = LocalizedStringBuilder::new("short_short").arg("d", dt).build();
        let out1 = clean(&s1.resolve(&map));
        // Should include date + time, but no seconds
        assert!(out1.contains("1/20/24"));
        assert!(out1.contains("3:30") || out1.contains("15:30"));
        assert!(!out1.contains(":00"));

        let s2 = LocalizedStringBuilder::new("short_medium").arg("d", dt).build();
        let out2 = clean(&s2.resolve(&map));
        // Should include seconds for medium
        assert!(out2.contains("1/20/24"));
        assert!(out2.contains("3:30") || out2.contains("15:30"));
        assert!(out2.contains(":00"));
    }

    #[test]
    fn time_only_formatting_branch() {
        // Covers the branch where only timeStyle is provided
        let source = r#"
short = { DATETIME($d, timeStyle: "short") }
medium = { DATETIME($d, timeStyle: "medium") }
        "#;
        let map = create_map(langid!("en-US"), source);

        let date = Date::from_calendar_date(2024, Month::January, 20).expect("Failed to create date");
        let time = Time::from_hms(15, 30, 0).expect("Failed to create time");
        let dt = date.with_time(time).assume_utc();

        let s1 = LocalizedStringBuilder::new("short").arg("d", dt).build();
        let out1 = clean(&s1.resolve(&map));
        assert!(out1.contains("3:30") || out1.contains("15:30"));
        // Should not include the date portion
        assert!(!out1.contains("2024"));
        // Short time should not include seconds
        assert!(!out1.contains(":00"));

        let s2 = LocalizedStringBuilder::new("medium").arg("d", dt).build();
        let out2 = clean(&s2.resolve(&map));
        assert!(out2.contains("3:30") || out2.contains("15:30"));
        assert!(!out2.contains("2024"));
        // Medium time should include seconds
        assert!(out2.contains(":00"));
    }

    #[test]
    fn date_only_long_style_branch() {
        // Covers the branch where only dateStyle is provided, using "long"
        let source = r#"
long = { DATETIME($d, dateStyle: "long") }
        "#;
        let map = create_map(langid!("en-US"), source);

        let date = Date::from_calendar_date(2024, Month::January, 20).expect("Failed to create date");
        let time = Time::from_hms(15, 30, 0).expect("Failed to create time");
        let dt = date.with_time(time).assume_utc();

        let s = LocalizedStringBuilder::new("long").arg("d", dt).build();
        let out = clean(&s.resolve(&map));

        // en-US long date should be a spelled month style (no slashes)
        assert!(out.contains("2024"));
        assert!(!out.contains('/'));
    }

    #[test]
    fn invalid_style_values_fall_back_to_default_date() {
        // Covers parse_date_length / parse_time_length returning None for invalid strings,
        // which should drop into the default "medium date" path.
        let source = r#"
bad = { DATETIME($d, dateStyle: "bogus", timeStyle: "bogus") }
        "#;
        let map = create_map(langid!("en-US"), source);

        let date = Date::from_calendar_date(2024, Month::January, 20).expect("Failed to create date");
        let time = Time::from_hms(15, 30, 0).expect("Failed to create time");
        let dt = date.with_time(time).assume_utc();

        let s = LocalizedStringBuilder::new("bad").arg("d", dt).build();
        assert_eq!(clean(&s.resolve(&map)), "Jan 20, 2024");
    }

    #[test]
    fn datetime_non_date_argument_passthrough() {
        // Covers the fallback path where DATETIME is given a non-date argument
        // and returns the value unchanged.
        let source = r#"
n = { DATETIME($d) }
        "#;
        let map = create_map(langid!("en-US"), source);

        let s = LocalizedStringBuilder::new("n").arg("d", 5.0).build();
        assert_eq!(clean(&s.resolve(&map)), "5");
    }

    #[test]
    fn datetime_string_parse_failure_passthrough() {
        // Covers the DATETIME branch where a string is provided but fails to parse as ISO.
        let source = r#"
dt = { DATETIME($d) }
        "#;
        let map = create_map(langid!("en-US"), source);

        let s = LocalizedStringBuilder::new("dt").arg("d", "not-a-date").build();
        assert_eq!(clean(&s.resolve(&map)), "not-a-date");
    }

    #[test]
    fn datetime_string_strict_mismatch_date_only_with_both_styles_passthrough() {
        let source = r#"
both = { DATETIME($d, dateStyle: "short", timeStyle: "short") }
        "#;
        let map = create_map(langid!("en-US"), source);

        let s = LocalizedStringBuilder::new("both").arg("d", "2024-01-20").build();
        assert_eq!(clean(&s.resolve(&map)), "2024-01-20");
    }

    #[test]
    fn datetime_string_strict_mismatch_time_only_with_date_style_passthrough() {
        let source = r#"
date = { DATETIME($t, dateStyle: "short") }
        "#;
        let map = create_map(langid!("en-US"), source);

        let s = LocalizedStringBuilder::new("date").arg("t", "15:30").build();
        assert_eq!(clean(&s.resolve(&map)), "15:30");
    }

    #[test]
    fn datetime_typed_date_and_time_inputs() {
        // Covers passing typed Date and Time directly through LocalizedArg (no strings).
        // This ensures we exercise the "AnyInput::Date" and "AnyInput::Time" paths.
        let source = r#"
date_short = { DATETIME($d, dateStyle: "short") }
time_short = { DATETIME($t, timeStyle: "short") }
time_medium = { DATETIME($t, timeStyle: "medium") }

date_bad = { DATETIME($t, dateStyle: "short") }
time_bad = { DATETIME($d, timeStyle: "short") }
        "#;

        let map = create_map(langid!("en-US"), source);

        let date = Date::from_calendar_date(2024, Month::January, 20).expect("Failed to create date");
        let time = Time::from_hms(15, 30, 0).expect("Failed to create time");

        // Date formats correctly when dateStyle requested
        let s1 = LocalizedStringBuilder::new("date_short").arg("d", date).build();
        assert_eq!(clean(&s1.resolve(&map)), "1/20/24");

        // Time formats correctly when timeStyle requested (short => no seconds)
        let s2 = LocalizedStringBuilder::new("time_short").arg("t", time).build();
        let out2 = clean(&s2.resolve(&map));
        assert!(out2.contains("3:30") || out2.contains("15:30"));
        assert!(!out2.contains(":00"));

        // Medium time includes seconds
        let s3 = LocalizedStringBuilder::new("time_medium").arg("t", time).build();
        let out3 = clean(&s3.resolve(&map));
        assert!(out3.contains("3:30") || out3.contains("15:30"));
        assert!(out3.contains(":00"));

        // Strict mismatches fall back to the original input value string representation.
        //
        // For typed Time, Fluent fallback formatting is typically the Time's Display impl.
        // For typed Date, it's typically "2024-01-20".
        let s4 = LocalizedStringBuilder::new("date_bad").arg("t", time).build();
        assert_eq!(clean(&s4.resolve(&map)), "15:30:00.0");

        let s5 = LocalizedStringBuilder::new("time_bad").arg("d", date).build();
        assert_eq!(clean(&s5.resolve(&map)), "2024-01-20");
    }

    #[test]
    fn fmt_grouping_always_strategy() {
        // Covers the ICU-specific "always" grouping strategy branch.
        let source = r#"
always = { NUMBER($val, useGrouping: "always") }
        "#;
        let map = create_map(langid!("en-US"), source);

        let s = LocalizedStringBuilder::new("always").arg("val", 10000.0).build();
        assert_eq!(clean(&s.resolve(&map)), "10,000");
    }

    #[test]
    fn number_uses_locale_specific_digits_ar_eg() {
        // ICU feature: locale-specific digits (Arabic-Indic for ar-EG)
        let source = "num = { NUMBER($val) }";
        let map = create_map(langid!("ar-EG"), source);

        let s = LocalizedStringBuilder::new("num").arg("val", 12345.0).build();
        let out = clean(&s.resolve(&map));

        // Arabic-Indic digits are U+0660..U+0669
        assert!(out.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)), "expected Arabic-Indic digits, got: {out}");

        // Should not contain ASCII digits if ICU is really localizing digits
        assert!(!out.chars().any(|c: char| c.is_ascii_digit()), "expected no ASCII digits, got: {out}");
    }

    #[test]
    fn number_uses_indian_grouping_pattern_hi_in() {
        // ICU feature: locale-specific grouping patterns (hi-IN uses 12,34,567 style)
        let source = "num = { NUMBER($val) }";
        let map = create_map(langid!("hi-IN"), source);

        let s = LocalizedStringBuilder::new("num").arg("val", 1234567.0).build();
        let out = clean(&s.resolve(&map));

        assert_eq!(out, "12,34,567");
    }
}

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;

    use crate::prelude::LocalizedString;

    fn roundtrip(ls: &LocalizedString) -> LocalizedString {
        let json = serde_json::to_string(ls).expect("serialize LocalizedString to JSON");
        serde_json::from_str(&json).expect("deserialize LocalizedString from JSON")
    }

    #[test]
    fn serde_roundtrip_basic_no_args() {
        let source = "hello = Hello World";
        let map = create_map(langid!("en-US"), source);

        let original = LocalizedStringBuilder::new("hello").build();
        assert_eq!(clean(&original.resolve(&map)), "Hello World");

        let decoded = roundtrip(&original);
        assert_eq!(clean(&decoded.resolve(&map)), "Hello World");
    }

    #[test]
    fn serde_roundtrip_missing_key_preserves_placeholder() {
        let source = "hello = Hello World";
        let map = create_map(langid!("en-US"), source);

        let original = LocalizedStringBuilder::new("missing_key").placeholder("Fallback").build();
        assert_eq!(clean(&original.resolve(&map)), "Fallback");

        let decoded = roundtrip(&original);
        assert_eq!(clean(&decoded.resolve(&map)), "Fallback");
    }

    #[test]
    fn serde_roundtrip_missing_key_without_placeholder_uses_key() {
        let source = "hello = Hello World";
        let map = create_map(langid!("en-US"), source);

        let original = LocalizedStringBuilder::new("missing_key").build();
        assert_eq!(clean(&original.resolve(&map)), "missing_key");

        let decoded = roundtrip(&original);
        assert_eq!(clean(&decoded.resolve(&map)), "missing_key");
    }

    #[test]
    fn serde_roundtrip_string_arg_borrowed_and_owned() {
        let source = "welcome = Welcome, { $name }!";
        let map = create_map(langid!("en-US"), source);

        // Borrowed &'static str
        let original1 = LocalizedStringBuilder::new("welcome").arg("name", "Alice").build();
        assert_eq!(clean(&original1.resolve(&map)), "Welcome, Alice!");

        let decoded1 = roundtrip(&original1);
        assert_eq!(clean(&decoded1.resolve(&map)), "Welcome, Alice!");

        // Owned String
        let original2 = LocalizedStringBuilder::new("welcome").arg("name", "Bob".to_string()).build();
        assert_eq!(clean(&original2.resolve(&map)), "Welcome, Bob!");

        let decoded2 = roundtrip(&original2);
        assert_eq!(clean(&decoded2.resolve(&map)), "Welcome, Bob!");
    }

    #[test]
    fn serde_roundtrip_number_arg() {
        let source = "items = You have { $count } items.";
        let map = create_map(langid!("en-US"), source);

        let original = LocalizedStringBuilder::new("items").arg("count", 5.0).build();
        assert_eq!(clean(&original.resolve(&map)), "You have 5 items.");

        let decoded = roundtrip(&original);
        assert_eq!(clean(&decoded.resolve(&map)), "You have 5 items.");
    }

    #[test]
    fn serde_roundtrip_multiple_args_and_repeated_resolution() {
        let source = r#"
msg = { $greeting }, { $name }! You have { $count } messages.
        "#;
        let map = create_map(langid!("en-US"), source);

        let original = LocalizedStringBuilder::new("msg")
            .arg("greeting", "Hello")
            .arg("name", "Alice")
            .arg("count", 3.0)
            .build();

        // Resolve multiple times (exercises cache behavior + arg tracking)
        assert_eq!(clean(&original.resolve(&map)), "Hello, Alice! You have 3 messages.");
        assert_eq!(clean(&original.resolve(&map)), "Hello, Alice! You have 3 messages.");

        let decoded = roundtrip(&original);

        // Resolve multiple times after deserialize too
        assert_eq!(clean(&decoded.resolve(&map)), "Hello, Alice! You have 3 messages.");
        assert_eq!(clean(&decoded.resolve(&map)), "Hello, Alice! You have 3 messages.");
    }

    #[test]
    fn serde_roundtrip_survives_map_locale_switch_and_different_maps() {
        let en_source = "hello = Hello";
        let fr_source = "hello = Bonjour";

        let en_map = create_map(langid!("en-US"), en_source);
        let fr_map = create_map(langid!("fr-FR"), fr_source);

        let original = LocalizedStringBuilder::new("hello").build();

        // Resolve against different maps/locales
        assert_eq!(clean(&original.resolve(&en_map)), "Hello");
        assert_eq!(clean(&original.resolve(&fr_map)), "Bonjour");

        let decoded = roundtrip(&original);

        assert_eq!(clean(&decoded.resolve(&en_map)), "Hello");
        assert_eq!(clean(&decoded.resolve(&fr_map)), "Bonjour");
    }

    #[cfg(feature = "icu")]
    mod icu_serde_tests {
        use super::*;
        use crate::{prelude::WeakVar, reactive::serde_impl::serde_scope};
        use serde::{Deserialize, Serialize};
        use time::{Date, Month, Time};

        fn scoped_to_string<T: Serialize>(value: &T) -> String {
            serde_scope(|| serde_json::to_string(value).expect("serialize to json"))
        }

        fn scoped_from_str<T: for<'de> Deserialize<'de>>(s: &str) -> T {
            serde_scope(|| serde_json::from_str(s).expect("deserialize from json"))
        }

        #[test]
        fn serde_roundtrip_datetime_typed_args() {
            let source = r#"
date = { DATETIME($d, dateStyle: "short") }
time = { DATETIME($t, timeStyle: "short") }
dt   = { DATETIME($dt, dateStyle: "short", timeStyle: "short") }
            "#;
            let map = create_map(langid!("en-US"), source);

            let date = Date::from_calendar_date(2024, Month::January, 20).unwrap();
            let time = Time::from_hms(15, 30, 0).unwrap();
            let dt = date.with_time(time).assume_utc();

            let original_date = LocalizedStringBuilder::new("date").arg("d", date).build();
            let original_time = LocalizedStringBuilder::new("time").arg("t", time).build();
            let original_dt = LocalizedStringBuilder::new("dt").arg("dt", dt).build();

            assert_eq!(clean(&original_date.resolve(&map)), "1/20/24");

            let t1 = clean(&original_time.resolve(&map));
            assert!(t1.contains("3:30") || t1.contains("15:30"));
            assert!(!t1.contains(":00"));

            let t2 = clean(&original_dt.resolve(&map));
            assert!(t2.contains("1/20/24"));
            assert!(t2.contains("3:30") || t2.contains("15:30"));

            let decoded_date = roundtrip(&original_date);
            let decoded_time = roundtrip(&original_time);
            let decoded_dt = roundtrip(&original_dt);

            assert_eq!(clean(&decoded_date.resolve(&map)), "1/20/24");

            let t3 = clean(&decoded_time.resolve(&map));
            assert!(t3.contains("3:30") || t3.contains("15:30"));
            assert!(!t3.contains(":00"));

            let t4 = clean(&decoded_dt.resolve(&map));
            assert!(t4.contains("1/20/24"));
            assert!(t4.contains("3:30") || t4.contains("15:30"));
        }

        #[test]
        fn localized_string_roundtrip_with_var_dependency_keeps_reactivity() {
            use crate::reactive::Var;

            // If you serialize a LocalizedString that contains WeakVars, but you don't
            // serialize the owning Var in the same serde graph + serde_scope,
            // then deserialization will allocate a fresh slot for the WeakVar, not initialize it,
            // and cleanup will kill it -> your arg becomes dead.

            #[derive(Serialize, Deserialize)]
            struct State {
                name: Var<String>,
                text: LocalizedString,
            }

            let source = "welcome = Welcome, { $name }!";
            let map = create_map(langid!("en-US"), source);

            let name = Var::new("Alice".into());

            let text = LocalizedStringBuilder::new("welcome").arg("name", name.downgrade()).build();

            assert_eq!(clean(&text.resolve(&map)), "Welcome, Alice!");

            let json = scoped_to_string(&State { name, text });
            let decoded: State = scoped_from_str(&json);

            // Still resolves correctly after roundtrip.
            assert_eq!(clean(&decoded.text.resolve(&map)), "Welcome, Alice!");

            // Mutate the decoded var -> localized string should update.
            decoded.name.set("Bob".into());
            assert_eq!(clean(&decoded.text.resolve(&map)), "Welcome, Bob!");
        }

        #[test]
        fn localized_string_and_external_weakvar_share_identity_after_roundtrip() {
            use crate::reactive::Var;

            #[derive(Serialize, Deserialize)]
            struct State {
                name: Var<String>,
                name_ref: WeakVar<String>,
                text: LocalizedString,
            }

            let source = "welcome = Welcome, { $name }!";
            let map = create_map(langid!("en-US"), source);

            let name = Var::new("Alice".to_string());
            let name_ref = name.downgrade();

            let text = LocalizedStringBuilder::new("welcome").arg("name", name_ref).build();

            let json = scoped_to_string(&State { name, name_ref, text });
            let decoded: State = scoped_from_str(&json);

            // Both references inside the decoded object should point at the same thing.
            assert_eq!(decoded.name.downgrade(), decoded.name_ref);

            // And the LocalizedString should be using the same handle too (indirectly).
            assert_eq!(clean(&decoded.text.resolve(&map)), "Welcome, Alice!");
        }
    }
}

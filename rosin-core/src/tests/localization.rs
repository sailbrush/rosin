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

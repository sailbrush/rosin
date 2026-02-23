//! Localization primitives built on top of [Project Fluent](https://projectfluent.org).
//!
//! This module provides two main types:
//!
//! - [`TranslationMap`]: a threadsafe collection of [`TranslationFile`]s keyed by locale.
//! - [`LocalizedString`]: a lazily resolved string with named arguments.
//!
//! ## Basic workflow
//!
//! 1. Load one or more Fluent translation files (`.ftl`) into a [`TranslationMap`].
//! 2. Set the active locale on the map.
//! 3. Build [`LocalizedString`] values with a message key and optional arguments.
//! 4. Pass [`LocalizedString`] to widgets that accept `impl Into<UIString>`
//!    or call [`LocalizedString::resolve`] to obtain the localized string for the current locale.
//!
//! ```ignore
//! let map = TranslationMap::new(langid!("en-US"))
//!     .add_translation(TranslationFile::from_file(vec![langid!("en-US")], "en-US.ftl")?)
//!     .add_translation(TranslationFile::from_file(vec![langid!("es-ES")], "es-ES.ftl")?);
//!
//! map.set_current_locale(langid!("es-ES"));
//!
//! let greeting = LocalizedStringBuilder::new("greeting")
//!     .arg("name", "Alice")
//!     .build();
//!
//! let text = greeting.resolve(&map);
//! assert_eq!(text, "Hola, Alice!");
//! ```
//!
//! ## Locale resolution
//!
//! `TranslationMap` resolves using the active locale exactly as set via [`TranslationMap::set_current_locale`].
//! This crate does not currently perform locale fallback/negotiation.
//!
//! If no translation file exists for the active locale, string resolution falls back to the placeholder (if set)
//! or the message key.
//!
//! ## Caching
//!
//! [`LocalizedString`] caches its resolved output for the current locale.
//! The cache is invalidated and recomputed when any of the following change:
//!
//! - the active locale
//! - the set of loaded translations (including reloads)
//! - any reactive arguments used by the string
//!
//! ## Missing messages and formatting errors
//!
//! When resolving a [`LocalizedString`]:
//!
//! - If the message key is not found, or the message exists but cannot be formatted,
//!   resolution falls back to the placeholder (if set), or the message key itself.
//! - If a formatting function (like `NUMBER` / `DATETIME`) cannot handle an argument,
//!   it returns the original value unchanged.
//!
//! ## Arguments
//!
//! Arguments set with [`LocalizedStringBuilder::arg`] become Fluent named arguments:
//!
//! ```ftl
//! greeting = Hello, { $name }!
//! items = You have { NUMBER($count) } items.
//! ```
//!
//! Supported argument values include:
//!
//! - strings (`&'static str`, `String`, `Cow<'static, str>`)
//! - numbers (`f64`)
//! - reactive variables (`WeakVar<T>`)
//! - `Date`, `Time`, `OffsetDateTime` *(requires the `icu` feature)*
//!
//! ## Custom formatting functions
//!
//! Translation files can call custom functions provided by this crate:
//!
//! - `NUMBER(...)` for numeric formatting
//! - `DATETIME(...)` for date/time formatting *(requires the `icu` feature)*
//!
//! Functions are used in Fluent files like this:
//!
//! ```ftl
//! score = Score: { NUMBER($score) }
//! updated = Updated: { DATETIME($when, dateStyle: "medium", timeStyle: "short") }
//! ```
//!
//! Named options are passed as `key: value` pairs:
//!
//! ```ftl
//! users = Users: { NUMBER($count, useGrouping: "false") }
//! ```
//!
//! ### `NUMBER(...)`
//!
//! The default `NUMBER` function formats numeric values and supports
//! a small set of options that mostly mirror common Intl/ECMA-402 names.
//!
//! Options:
//!
//! - `useGrouping`
//!   - `"false"` disables grouping separators.
//!
//! - `minimumIntegerDigits`
//!   - Pads the integer portion with leading zeroes.
//!
//! - `minimumFractionDigits`
//!   - Pads the fractional portion with trailing zeroes.
//!
//! - `maximumFractionDigits`
//!   - Rounds to at most this many fractional digits.
//!
//! - `minimumSignificantDigits` / `maximumSignificantDigits`
//!   - Significant digit formatting.
//!   - If either significant digit option is present, it takes precedence over fraction digit options.
//!
//! #### Default formatter *(no `icu` feature)*
//!
//! Without `icu`, `NUMBER(...)` is implemented with a simplified formatter.
//!
//! - Decimal separator is always `.`
//! - Grouping uses `,` when enabled
//! - Output is ASCII (not locale-aware)
//! - Supports all `NUMBER(...)` options listed above
//!
//! Examples:
//!
//! ```ftl
//! # Basic
//! n-basic = { NUMBER($n) }
//!
//! # Force exactly 2 decimals
//! n-2dp = { NUMBER($n, minimumFractionDigits: 2, maximumFractionDigits: 2) }
//! ```
//!
//! #### What `icu` adds to `NUMBER(...)`
//!
//! Enabling the `icu` feature switches `NUMBER(...)` to ICU-backed formatting and adds true locale-aware output:
//!
//! - Locale-appropriate decimal separator and grouping separator
//! - Locale-appropriate grouping patterns
//! - `useGrouping: "always"` support (forces grouping where ICU supports it)
//! - Uses locale-appropriate digits (not just 0–9). For example, in `ar-EG`, `{ NUMBER(12345) }` may render as `١٢٣٤٥`.
//!
//! **Note:** ICU currency formatting is still considered experimental upstream, so currency formatting
//! is not supported yet.
//!
//! ### `DATETIME(...)` *(requires `icu`)*
//!
//! The `DATETIME` function formats date/time values using ICU locale rules.
//! It is only available when the `icu` feature is enabled.
//!
//! Supported inputs are:
//!
//! - `time::Date`
//! - `time::Time`
//! - `time::OffsetDateTime`
//! - A string in ISO-8601 date-time form: `YYYY-MM-DDTHH:MM:SS`
//!
//! Options:
//!
//! - `dateStyle`: `"long" | "medium" | "short"`
//! - `timeStyle`: `"long" | "medium" | "short"`
//!
//! What `"long" | "medium" | "short"` mean:
//!
//! These map to ICU "length" styles. The exact output is locale-specific, but the intent is:
//!
//! - `dateStyle: "long"`
//!   - A more verbose, more human-friendly date format.
//!   - Typically uses month names and more words (for example `January 15, 2026` in `en-US`).
//!
//! - `dateStyle: "medium"`
//!   - A compact but still readable date format.
//!   - Often uses abbreviated month names (for example `Jan 15, 2026` in `en-US`).
//!
//! - `dateStyle: "short"`
//!   - The most compact date format.
//!   - Typically numeric (for example `1/15/26` in `en-US`).
//!
//! - `timeStyle: "short"`
//!   - Hours + minutes only.
//!   - Example: `9:30 AM` (seconds omitted).
//!
//! - `timeStyle: "medium"` / `timeStyle: "long"`
//!   - Includes seconds.
//!   - Example: `9:30:00 AM`.
//!
//!   **Note:** This crate currently does not include time zone names/offsets in the output,
//!   so `"long"` time is usually the same as `"medium"`.
//!
//! Behavior:
//!
//! - `dateStyle` only → formats date (or the date part of a datetime)
//! - `timeStyle` only → formats time (or the time part of a datetime)
//! - both → formats full date + time *(requires a datetime input)*
//! - neither → defaults to `dateStyle: "medium"`
//!
//! **Note:** Time zone names and offsets are not currently formatted. `OffsetDateTime` values are formatted
//! using only their date and time components.
//!
//! Examples:
//!
//! ```ftl
//! # Date only
//! dt-date = { DATETIME($when, dateStyle: "long") }
//!
//! # Time only
//! dt-time = { DATETIME($when, timeStyle: "short") }
//!
//! # Date + time (OffsetDateTime recommended)
//! dt-both = { DATETIME($when, dateStyle: "medium", timeStyle: "short") }
//! ```
//!
//! ### `icu` feature
//!
//! When the `icu` feature is enabled, the crate uses the `icu` and `time` crates to provide:
//!
//! - More complete locale-aware number formatting for `NUMBER(...)`
//! - Date and time formatting via `DATETIME(...)`
//!
//! With `icu` enabled, this crate also re-exports:
//!
//! - `time::Date`
//! - `time::Time`
//! - `time::OffsetDateTime`
//!
//! so you can pass them as arguments without adding a direct dependency on the `time` crate.
//!
//! ### `serde` feature
//!
//! When the `serde` feature is enabled, [`LocalizedString`] can be serialized and deserialized.
//!
//! ### Security
//!
//! Currently, the fluent crate can be coerced into panicking, so it's not recommended to use untrusted localization files.

use std::{borrow::Cow, collections::HashMap, fmt, fs, path::Path, sync::Arc, time::Instant};

use fluent_bundle::{FluentArgs, FluentResource, FluentValue, concurrent::FluentBundle};
use log::error;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use sys_locale::get_locale;
use unic_langid::LanguageIdentifier;

#[cfg(feature = "icu")]
use fixed_decimal::{Decimal, FloatPrecision};
#[cfg(feature = "icu")]
use fluent_bundle::types::FluentType;
#[cfg(feature = "icu")]
use icu::{
    calendar::Iso,
    datetime::{
        DateTimeFormatter,
        fieldsets::{
            self,
            enums::{DateAndTimeFieldSet, DateFieldSet, TimeFieldSet},
        },
        options::Length,
    },
    decimal::{
        DecimalFormatter,
        options::{DecimalFormatterOptions, GroupingStrategy},
    },
};
#[cfg(feature = "icu")]
use time::{Date, OffsetDateTime, Time};

#[cfg(feature = "icu")]
use std::str::FromStr;

use crate::{
    prelude::*,
    reactive::{VarKey, VarReadGuard},
    util::ResourceInfo,
};

struct TranslationMapInner {
    last_loaded: Instant,
    current_locale: LanguageIdentifier,
    translations: HashMap<LanguageIdentifier, TranslationFile>,
}

/// The global set of all translations for an application.
///
/// It maps the current locale to a [`TranslationFile`].
///
/// The contents are stored in an [`Arc<T>`] so it's cheap to clone.
#[derive(Clone)]
pub struct TranslationMap {
    // we need both change tracking and shared ownership
    inner: Arc<Var<TranslationMapInner>>,
}

impl Default for TranslationMap {
    /// Creates a TranslationMap with the system's locale. Defaults to "en-US" if getting the system's locale fails.
    fn default() -> Self {
        Self::new(get_locale().map_or_else(|| unic_langid::langid!("en-US"), |locale| locale.parse().unwrap_or(unic_langid::langid!("en-US"))))
    }
}

impl TranslationMap {
    /// Creates a new TranslationMap.
    pub fn new(current_locale: LanguageIdentifier) -> Self {
        Self {
            inner: Arc::new(Var::new(TranslationMapInner {
                last_loaded: Instant::now(),
                current_locale,
                translations: HashMap::new(),
            })),
        }
    }

    /// Adds a new translation file to the map so it can be used by the application.
    pub fn add_translation(self, file: TranslationFile) -> Self {
        let mut guard = self.inner.write();
        // We can safely access index 0 because constructors guarantee locales is not empty
        guard.translations.insert(file.bundle.locales[0].clone(), file);
        guard.last_loaded = Instant::now();
        drop(guard);
        self
    }

    /// Returns a [`FluentBundle`] for the requested locale, if available.
    pub fn get_bundle(&self, locale: &LanguageIdentifier) -> Option<VarReadGuard<'_, FluentBundle<FluentResource>>> {
        VarReadGuard::try_map(self.inner.read(), |inner| inner.translations.get(locale).map(|f| &f.bundle)).ok()
    }

    /// Returns the locale that will be used to resolve strings.
    pub fn get_current_locale(&self) -> VarReadGuard<'_, LanguageIdentifier> {
        VarReadGuard::map(self.inner.read(), |inner| &inner.current_locale)
    }

    /// Sets the locale that will be used to resolve strings.
    pub fn set_current_locale(&mut self, locale: LanguageIdentifier) {
        self.inner.write().current_locale = locale;
    }

    /// Reloads any files that have been modified since they were last loaded.
    ///
    /// Returns `true` if any of the files were successfully reloaded.
    ///
    /// ## Errors
    ///
    /// The method will return a boolean irrelevant of errors, but in case of errors,
    /// the `Err` variant will also contain a Vec of any io errors encountered.
    pub fn reload(&mut self) -> Result<bool, (bool, Vec<std::io::Error>)> {
        let mut guard = self.inner.write();

        let mut reloaded = false;
        let mut errors = Vec::new();

        for file in guard.translations.values_mut() {
            match file.reload() {
                Ok(did_load) => reloaded |= did_load,
                Err(error) => errors.push(error),
            }
        }

        if reloaded {
            guard.last_loaded = Instant::now();
        } else {
            // nothing changed on disk, don't mark as changed
            guard.cancel_change();
        }

        if !errors.is_empty() { Err((reloaded, errors)) } else { Ok(reloaded) }
    }
}

/// A collection of localization messages for a single locale.
///
/// See <https://projectfluent.org> for a description of the syntax of Fluent files.
pub struct TranslationFile {
    info: Option<ResourceInfo>,
    bundle: FluentBundle<FluentResource>,
}

impl TranslationFile {
    /// Loads translations from a string. The first element in `locales` should be the language this file represents, and will be used to
    /// determine the correct plural rules for this file. You can optionally provide extra languages in the list; they will be used as
    /// fallback date and time formatters if a formatter for the primary language is unavailable.
    pub fn from_str(locales: Vec<LanguageIdentifier>, text: &str) -> Result<Self, std::io::Error> {
        let bundle = Self::make_bundle(locales, text.to_string())?;
        Ok(Self { info: None, bundle })
    }

    /// Loads translations from a file. The first element in `locales` should be the language this file represents, and will be used to
    /// determine the correct plural rules for this file. You can optionally provide extra languages in the list; they will be used as
    /// fallback date and time formatters if a formatter for the primary language is unavailable.
    pub fn from_file(locales: Vec<LanguageIdentifier>, path: impl AsRef<Path>) -> Result<TranslationFile, std::io::Error> {
        let path_buf = path.as_ref().canonicalize()?;
        let text = fs::read_to_string(&path_buf)?;
        let bundle = Self::make_bundle(locales, text)?;
        let info = Some(ResourceInfo {
            last_modified: fs::metadata(&path_buf)?.modified()?,
            path: path_buf.clone(),
        });

        Ok(Self { info, bundle })
    }

    pub(crate) fn reload(&mut self) -> Result<bool, std::io::Error> {
        if let Some(ref mut resource_info) = self.info {
            let current_modified_time = fs::metadata(&resource_info.path)?.modified()?;

            if current_modified_time > resource_info.last_modified {
                // File has been modified, reload it
                let text = fs::read_to_string(&resource_info.path)?;
                self.bundle = Self::make_bundle(self.bundle.locales.clone(), text)?;

                resource_info.last_modified = current_modified_time;

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Shared logic for creating a FluentBundle from text.
    fn make_bundle(locales: Vec<LanguageIdentifier>, text: String) -> Result<FluentBundle<FluentResource>, std::io::Error> {
        if locales.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Locales list cannot be empty"));
        }

        let resource = match FluentResource::try_new(text) {
            Ok(res) => res,
            Err((res, error_list)) => {
                for error in error_list {
                    error!("{error}");
                }
                res
            }
        };
        let mut bundle = FluentBundle::new_concurrent(locales.clone());

        Self::add_custom_formatters(&mut bundle, &locales);

        if let Err(error_list) = bundle.add_resource(resource) {
            for error in error_list {
                error!("{error}");
            }
        }
        Ok(bundle)
    }

    #[cfg(not(feature = "icu"))]
    fn add_custom_formatters(bundle: &mut FluentBundle<FluentResource>, _: &[LanguageIdentifier]) {
        let _ = bundle.add_function("NUMBER", |args, named_args| {
            let value = match args.first() {
                Some(FluentValue::Number(n)) => n.value,
                Some(v) => return v.clone(),
                None => return FluentValue::Error,
            };

            // If we can't handle it as a number, return it unchanged.
            // This also avoids weird padding/grouping behavior for NaN/Infinity.
            if !value.is_finite() {
                return FluentValue::from(value.to_string());
            }

            let get_opt = |name: &str| -> Option<i64> {
                match named_args.get(name) {
                    Some(FluentValue::Number(n)) if n.value.is_finite() => Some(n.value.trunc() as i64),
                    _ => None,
                }
            };

            const MAX_FRAC: i64 = 20;
            const MAX_SIG: i64 = 21;
            const MAX_MIN_INT: i64 = 308;

            let use_grouping = named_args
                .get("useGrouping")
                .map(|v| match v {
                    FluentValue::String(s) => s.as_ref() != "false",
                    _ => true,
                })
                .unwrap_or(true);

            let min_integer_digits = get_opt("minimumIntegerDigits").map(|v| v.clamp(1, MAX_MIN_INT) as usize).unwrap_or(1);
            let max_fraction_digits = get_opt("maximumFractionDigits").map(|v| v.clamp(0, MAX_FRAC) as usize);

            let mut min_fraction_digits = get_opt("minimumFractionDigits").map(|v| v.clamp(0, MAX_FRAC) as usize).unwrap_or(0);
            if let Some(maxf) = max_fraction_digits {
                min_fraction_digits = min_fraction_digits.min(maxf);
            }

            let min_sig_raw = get_opt("minimumSignificantDigits").map(|v| v.clamp(1, MAX_SIG) as usize);
            let max_sig_raw = get_opt("maximumSignificantDigits").map(|v| v.clamp(1, MAX_SIG) as usize);

            let sig_mode = min_sig_raw.is_some() || max_sig_raw.is_some();

            let text = if sig_mode {
                let mut min_sig = min_sig_raw.unwrap_or(1);
                let max_sig = max_sig_raw.unwrap_or(21);
                if max_sig < min_sig {
                    min_sig = max_sig;
                }

                // Only maxSig rounds. (If user didn't specify maxSig explicitly, don't round.)
                let rounded = if let Some(user_max) = max_sig_raw {
                    if value != 0.0 && value.is_finite() {
                        let log10 = value.abs().log10();
                        let magnitude = if log10.is_finite() { log10.floor() as isize } else { 0 };

                        // shift decimal so we can round to `user_max` sig digits
                        let power = -(magnitude - (user_max as isize - 1));
                        let factor = 10f64.powi(power as i32);

                        // If scaling over/underflows, skip rounding rather than producing NaN.
                        if !factor.is_finite() || factor == 0.0 {
                            value
                        } else {
                            let r = (value * factor).round() / factor;
                            if r.is_finite() { r } else { value }
                        }
                    } else {
                        value
                    }
                } else {
                    value
                };

                // Avoid scientific notation when maxSig is specified by forcing fixed-point formatting
                let mut s = if let Some(user_max) = max_sig_raw {
                    let abs = rounded.abs();
                    if abs != 0.0 && abs.is_finite() {
                        let magnitude = abs.log10().floor() as isize;
                        let frac = ((user_max as isize - 1) - magnitude).max(0) as usize;
                        format!("{:.*}", frac, rounded)
                    } else {
                        rounded.to_string()
                    }
                } else {
                    // minSig only: don't truncate/round
                    rounded.to_string()
                };

                // If this ends up in scientific notation, don't try to manipulate digits.
                if s.contains('e') || s.contains('E') {
                    return FluentValue::from(s);
                }

                // maxSig must not force trailing zeros
                if s.as_bytes().contains(&b'.') {
                    while s.ends_with('0') {
                        s.pop();
                    }
                    if s.ends_with('.') {
                        s.pop();
                    }
                }

                // minSig pads (does not change value)
                // Count significant digits: strip sign + '.', trim leading zeros
                let mut seen_nonzero = false;
                let mut current_sig = 0usize;

                for &b in s.as_bytes() {
                    if b.is_ascii_digit() {
                        if !seen_nonzero {
                            if b != b'0' {
                                seen_nonzero = true;
                                current_sig += 1;
                            }
                        } else {
                            current_sig += 1;
                        }
                    }
                }

                if current_sig == 0 {
                    // number is effectively 0 -> "0" or "0.00..."
                    if min_sig <= 1 {
                        s.clear();
                        s.push('0');
                    } else {
                        s.clear();
                        s.push('0');
                        s.push('.');
                        for _ in 0..(min_sig - 1) {
                            s.push('0');
                        }
                    }
                } else if current_sig < min_sig {
                    let needed = min_sig - current_sig;
                    if !s.as_bytes().contains(&b'.') {
                        s.push('.');
                    }
                    for _ in 0..needed {
                        s.push('0');
                    }
                }

                s
            } else {
                // Fraction mode
                if let Some(max_frac) = max_fraction_digits {
                    let mut s = format!("{0:.1$}", value, max_frac);

                    // trim if we were just limiting, not padding
                    if s.as_bytes().contains(&b'.') {
                        while s.ends_with('0') {
                            s.pop();
                        }
                        if s.ends_with('.') {
                            s.pop();
                        }
                    }

                    // ensure minFraction padding
                    if min_fraction_digits > 0 {
                        let dot = s.find('.');
                        let have = match dot {
                            Some(d) => s.len().saturating_sub(d + 1),
                            None => 0,
                        };

                        if have < min_fraction_digits {
                            if dot.is_none() {
                                s.push('.');
                            }
                            for _ in 0..(min_fraction_digits - have) {
                                s.push('0');
                            }
                        }
                    }

                    s
                } else {
                    // No maxFraction: default shortest formatting
                    let mut s = value.to_string();

                    // If we got scientific notation, don't try to group/pad it.
                    if s.contains('e') || s.contains('E') {
                        return FluentValue::from(s);
                    }

                    // If minFraction is set, pad with zeros but don't round/truncate.
                    if min_fraction_digits > 0 {
                        let dot = s.find('.');
                        let have = match dot {
                            Some(d) => s.len().saturating_sub(d + 1),
                            None => 0,
                        };

                        if have < min_fraction_digits {
                            if dot.is_none() {
                                s.push('.');
                            }
                            for _ in 0..(min_fraction_digits - have) {
                                s.push('0');
                            }
                        }
                    }

                    s
                }
            };

            // If we ever ended up with scientific notation, leave it alone.
            if text.contains('e') || text.contains('E') {
                return FluentValue::from(text);
            }

            let (int_slice, frac_slice) = if let Some(dot) = text.find('.') {
                (&text[..dot], &text[dot..])
            } else {
                (text.as_str(), "")
            };

            // Pad integer to minimumIntegerDigits
            let neg = int_slice.starts_with('-');
            let digits = if neg { &int_slice[1..] } else { int_slice };
            let needed = min_integer_digits.saturating_sub(digits.len());

            let mut int_part = String::with_capacity(int_slice.len() + needed);
            if neg {
                int_part.push('-');
            }
            for _ in 0..needed {
                int_part.push('0');
            }
            int_part.push_str(digits);

            // Grouping: commas every 3 digits
            if use_grouping {
                let neg = int_part.starts_with('-');
                let start = if neg { 1 } else { 0 };
                let digits = &int_part[start..];

                if digits.len() > 3 {
                    let mut grouped = String::with_capacity(int_part.len() + (digits.len() / 3));
                    if neg {
                        grouped.push('-');
                    }

                    let offset = digits.len() % 3;

                    if offset > 0 {
                        grouped.push_str(&digits[..offset]);
                        grouped.push(',');
                    }

                    for (i, b) in digits[offset..].bytes().enumerate() {
                        if i > 0 && i % 3 == 0 {
                            grouped.push(',');
                        }
                        grouped.push(b as char);
                    }

                    int_part = grouped;
                }
            }

            if !frac_slice.is_empty() {
                int_part.push_str(frac_slice);
            }

            FluentValue::from(int_part)
        });
    }

    #[cfg(feature = "icu")]
    fn add_custom_formatters(bundle: &mut FluentBundle<FluentResource>, locales: &[LanguageIdentifier]) {
        use icu::locale::locale;

        let icu_locale = locales.first().and_then(|l| l.to_string().parse().ok()).unwrap_or(locale!("en-US"));

        let _ = bundle.add_function("NUMBER", {
            let icu_locale = icu_locale.clone();
            move |args, named_args| {
                let num_value = match args.first() {
                    Some(FluentValue::Number(n)) => n,
                    Some(other) => return other.clone(),
                    None => return FluentValue::Error,
                };

                // If we can't handle it as a number, return it unchanged.
                if !num_value.value.is_finite() {
                    return FluentValue::from(num_value.value.to_string());
                }

                let mut options = DecimalFormatterOptions::default();

                // Handle useGrouping
                if let Some(FluentValue::String(s)) = named_args.get("useGrouping") {
                    if s.as_ref() == "false" {
                        options.grouping_strategy = Some(GroupingStrategy::Never);
                    } else if s.as_ref() == "always" {
                        options.grouping_strategy = Some(GroupingStrategy::Always);
                    }
                }

                let mut decimal = match Decimal::try_from_f64(num_value.value, FloatPrecision::RoundTrip) {
                    Ok(d) => d,
                    Err(_) => return FluentValue::from(num_value.value.to_string()),
                };

                let get_opt = |name: &str| -> Option<i64> {
                    match named_args.get(name) {
                        Some(FluentValue::Number(n)) if n.value.is_finite() => Some(n.value.trunc() as i64),
                        _ => None,
                    }
                };

                // These ranges mirror common Intl/ECMA-402 expectations.
                const MAX_FRAC: i64 = 20;
                const MAX_SIG: i64 = 21;
                const MAX_MIN_INT: i64 = 308;

                let min_sig_raw = get_opt("minimumSignificantDigits").map(|v| v.clamp(1, MAX_SIG) as i16);
                let max_sig_raw = get_opt("maximumSignificantDigits").map(|v| v.clamp(1, MAX_SIG) as i16);

                // Enforce min <= max when both are present
                let (min_sig, max_sig) = match (min_sig_raw, max_sig_raw) {
                    (Some(a), Some(b)) => (Some(a.min(b)), Some(b)),
                    other => other,
                };

                let mut sig_applied = false;
                let val_abs = num_value.value.abs();

                // Significant Digits Logic
                // Note: Max significant digits limits precision. If the result ends in zeros that are not required
                // by minimumSignificantDigits, they should be trimmed.
                if let Some(max) = max_sig {
                    let magnitude = if val_abs != 0.0 { val_abs.log10().floor() as i16 } else { 0 };
                    let position = magnitude - (max - 1);
                    decimal.round(position);
                    decimal.trim_end(); // Remove zeros that might have been added or kept by rounding logic
                    sig_applied = true;
                }

                if let Some(min) = min_sig {
                    let magnitude = if val_abs != 0.0 { val_abs.log10().floor() as i16 } else { 0 };
                    let position = magnitude - (min - 1);
                    decimal.pad_end(position); // Ensure we have at least this many significant digits
                    sig_applied = true;
                }

                // Fraction Digits Logic
                if !sig_applied {
                    let min_frac_raw = get_opt("minimumFractionDigits").map(|v| v.clamp(0, MAX_FRAC) as i16);
                    let max_frac_raw = get_opt("maximumFractionDigits").map(|v| v.clamp(0, MAX_FRAC) as i16);

                    // Enforce min <= max when both are present
                    let (min_frac, max_frac) = match (min_frac_raw, max_frac_raw) {
                        (Some(a), Some(b)) => (Some(a.min(b)), Some(b)),
                        other => other,
                    };

                    // Min Fraction: Pad
                    if let Some(min_frac) = min_frac {
                        decimal.pad_end(-min_frac);
                    }

                    // Max Fraction: Round (truncate precision), but don't force padding if the number is already shorter.
                    // FixedDecimal::round() will pad if the number is less precise than the round magnitude.
                    // We check magnitude_range start to avoid this.
                    if let Some(max_frac) = max_frac {
                        let limit = -max_frac;
                        // Only round if the number currently extends beyond the limit (i.e. is more precise)
                        if *decimal.magnitude_range().start() < limit {
                            decimal.round(limit);
                        }
                    }
                }

                // Integer Digits
                if let Some(min_int) = get_opt("minimumIntegerDigits").map(|v| v.clamp(1, MAX_MIN_INT) as i16) {
                    decimal.pad_start(min_int);
                }

                // Create Formatter
                let formatter: DecimalFormatter = match DecimalFormatter::try_new(icu_locale.clone().into(), options) {
                    Ok(fmt) => fmt,
                    Err(_) => return FluentValue::from(num_value.value.to_string()),
                };

                FluentValue::from(formatter.format(&decimal).to_string())
            }
        });

        let _ = bundle.add_function("DATETIME", {
            let icu_locale = icu_locale.clone();
            move |args, named_args| {
                let fallback_value = match args.first() {
                    Some(v) => v.clone(),
                    None => return FluentValue::Error,
                };

                let parse_date_length = |val: &FluentValue| -> Option<Length> {
                    if let FluentValue::String(s) = val {
                        match s.as_ref() {
                            "long" => Some(Length::Long),
                            "medium" => Some(Length::Medium),
                            "short" => Some(Length::Short),
                            _ => None,
                        }
                    } else {
                        None
                    }
                };

                let parse_time_length = |val: &FluentValue| -> Option<Length> {
                    if let FluentValue::String(s) = val {
                        match s.as_ref() {
                            "long" => Some(Length::Long),
                            "medium" => Some(Length::Medium),
                            "short" => Some(Length::Short),
                            _ => None,
                        }
                    } else {
                        None
                    }
                };

                let date_style = named_args.get("dateStyle").and_then(parse_date_length);
                let time_style = named_args.get("timeStyle").and_then(parse_time_length);

                let effective_date_style = date_style.or_else(|| if time_style.is_none() { Some(Length::Medium) } else { None });

                enum DateOrTime {
                    Date(icu::calendar::Date<Iso>),
                    Time(icu::datetime::input::Time),
                    DateTime(icu::datetime::input::DateTime<Iso>),
                }

                let to_icu_date =
                    |d: Date| -> Option<icu::calendar::Date<Iso>> { icu::calendar::Date::try_new_iso(d.year(), u8::from(d.month()), d.day()).ok() };

                let to_icu_time = |t: Time| -> Option<icu::datetime::input::Time> {
                    icu::datetime::input::Time::try_new(t.hour(), t.minute(), t.second(), t.nanosecond()).ok()
                };

                let to_icu_datetime = |dt: OffsetDateTime| -> Option<icu::datetime::input::DateTime<Iso>> {
                    let d = dt.date();
                    let t = dt.time();
                    let date = to_icu_date(d)?;
                    let time = to_icu_time(t)?;
                    Some(icu::datetime::input::DateTime { date, time })
                };

                let input: DateOrTime = match args.first() {
                    Some(FluentValue::Custom(custom)) => {
                        if let Some(arg) = custom.as_any().downcast_ref::<LocalizedArg>() {
                            match arg {
                                LocalizedArg::ConstDate(d) => {
                                    let date = match to_icu_date(*d) {
                                        Some(v) => v,
                                        None => return fallback_value,
                                    };
                                    DateOrTime::Date(date)
                                }
                                LocalizedArg::VarDate(v) => {
                                    let date = match v.get().and_then(to_icu_date) {
                                        Some(v) => v,
                                        None => return fallback_value,
                                    };
                                    DateOrTime::Date(date)
                                }
                                LocalizedArg::ConstTime(t) => {
                                    let time = match to_icu_time(*t) {
                                        Some(v) => v,
                                        None => return fallback_value,
                                    };
                                    DateOrTime::Time(time)
                                }
                                LocalizedArg::VarTime(v) => {
                                    let time = match v.get().and_then(to_icu_time) {
                                        Some(v) => v,
                                        None => return fallback_value,
                                    };
                                    DateOrTime::Time(time)
                                }
                                LocalizedArg::ConstDateTime(dt) => {
                                    let dt = match to_icu_datetime(*dt) {
                                        Some(v) => v,
                                        None => return fallback_value,
                                    };
                                    DateOrTime::DateTime(dt)
                                }
                                LocalizedArg::VarDateTime(v) => {
                                    let dt = match v.get().and_then(to_icu_datetime) {
                                        Some(v) => v,
                                        None => return fallback_value,
                                    };
                                    DateOrTime::DateTime(dt)
                                }

                                // If DATETIME gets a non-date/time LocalizedArg (string, number, etc), just fallback.
                                _ => return fallback_value,
                            }
                        } else {
                            // Unknown custom type: pass through unchanged.
                            return fallback_value;
                        }
                    }
                    Some(FluentValue::String(s)) => match icu::datetime::input::DateTime::<Iso>::from_str(s.as_ref()) {
                        Ok(dt) => DateOrTime::DateTime(dt),
                        Err(_) => return FluentValue::from(s.clone()),
                    },
                    Some(v) => return v.clone(),
                    None => return FluentValue::Error,
                };

                // Format according to requested styles
                let formatted = match (effective_date_style, time_style) {
                    (Some(date_len), Some(time_len)) => {
                        let dt = match &input {
                            DateOrTime::DateTime(dt) => dt,
                            _ => return fallback_value,
                        };

                        let ymd = fieldsets::YMD::for_length(date_len);
                        let ymdt = match time_len {
                            Length::Short => ymd.with_time_hm(),
                            Length::Medium | Length::Long => ymd.with_time_hms(),
                            _ => ymd.with_time_hms(),
                        };

                        match DateTimeFormatter::<DateAndTimeFieldSet>::try_new(icu_locale.clone().into(), DateAndTimeFieldSet::YMDT(ymdt)) {
                            Ok(fmt) => fmt.format(dt).to_string(),
                            Err(_) => return fallback_value,
                        }
                    }
                    (Some(date_len), None) => {
                        let ymd = fieldsets::YMD::for_length(date_len);

                        let fmt = match DateTimeFormatter::<DateFieldSet>::try_new(icu_locale.clone().into(), DateFieldSet::YMD(ymd)) {
                            Ok(fmt) => fmt,
                            Err(_) => return fallback_value,
                        };

                        match &input {
                            DateOrTime::Date(date) => fmt.format(date).to_string(),
                            DateOrTime::DateTime(dt) => fmt.format(dt).to_string(),
                            DateOrTime::Time(_) => return fallback_value,
                        }
                    }
                    (None, Some(time_len)) => {
                        let tf = match time_len {
                            Length::Short => fieldsets::T::hm().with_length(time_len),
                            Length::Medium | Length::Long => fieldsets::T::hms().with_length(time_len),
                            _ => fieldsets::T::hms().with_length(time_len),
                        };

                        let fmt = match DateTimeFormatter::<TimeFieldSet>::try_new(icu_locale.clone().into(), TimeFieldSet::T(tf)) {
                            Ok(fmt) => fmt,
                            Err(_) => return fallback_value,
                        };

                        match &input {
                            DateOrTime::Time(time) => fmt.format(time).to_string(),
                            DateOrTime::DateTime(dt) => fmt.format(dt).to_string(),
                            DateOrTime::Date(_) => return fallback_value,
                        }
                    }
                    (None, None) => return fallback_value,
                };

                FluentValue::from(formatted)
            }
        });
    }
}

/// A concrete argument type for localization.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LocalizedArg {
    ConstString(Cow<'static, str>),
    ConstNumber(f64),
    VarString(WeakVar<String>),
    VarNumber(WeakVar<f64>),
    #[cfg(feature = "icu")]
    ConstTime(Time),
    #[cfg(feature = "icu")]
    VarTime(WeakVar<Time>),
    #[cfg(feature = "icu")]
    ConstDate(Date),
    #[cfg(feature = "icu")]
    VarDate(WeakVar<Date>),
    #[cfg(feature = "icu")]
    ConstDateTime(OffsetDateTime),
    #[cfg(feature = "icu")]
    VarDateTime(WeakVar<OffsetDateTime>),
}

#[cfg(feature = "icu")]
impl FluentType for LocalizedArg {
    fn duplicate(&self) -> Box<dyn FluentType + Send> {
        Box::new(self.clone())
    }

    fn as_string(&self, _intls: &intl_memoizer::IntlLangMemoizer) -> Cow<'static, str> {
        // Fallback string representation. In normal use, the DATETIME formatter
        // formats the date variants directly.
        match self {
            LocalizedArg::ConstString(s) => s.clone(),
            LocalizedArg::ConstNumber(n) => Cow::Owned(n.to_string()),
            LocalizedArg::VarString(v) => v.get().map(|val| Cow::Owned(val.clone())).unwrap_or_default(),
            LocalizedArg::VarNumber(v) => v.get().map(|val| Cow::Owned(val.to_string())).unwrap_or_default(),
            LocalizedArg::ConstTime(t) => Cow::Owned(format!("{}", t)),
            LocalizedArg::VarTime(v) => v.get().map(|t| Cow::Owned(format!("{}", t))).unwrap_or_default(),
            LocalizedArg::ConstDate(d) => Cow::Owned(format!("{}", d)),
            LocalizedArg::VarDate(v) => v.get().map(|d| Cow::Owned(format!("{}", d))).unwrap_or_default(),
            LocalizedArg::ConstDateTime(dt) => Cow::Owned(format!("{}", dt)),
            LocalizedArg::VarDateTime(v) => v.get().map(|dt| Cow::Owned(format!("{}", dt))).unwrap_or_default(),
        }
    }

    fn as_string_threadsafe(&self, _intls: &intl_memoizer::concurrent::IntlLangMemoizer) -> Cow<'static, str> {
        // Same as `as_string`, but for the threadsafe memoizer.
        match self {
            LocalizedArg::ConstString(s) => s.clone(),
            LocalizedArg::ConstNumber(n) => Cow::Owned(n.to_string()),
            LocalizedArg::VarString(v) => v.get().map(|val| Cow::Owned(val.clone())).unwrap_or_default(),
            LocalizedArg::VarNumber(v) => v.get().map(|val| Cow::Owned(val.to_string())).unwrap_or_default(),
            LocalizedArg::ConstTime(t) => Cow::Owned(format!("{}", t)),
            LocalizedArg::VarTime(v) => v.get().map(|t| Cow::Owned(format!("{}", t))).unwrap_or_default(),
            LocalizedArg::ConstDate(d) => Cow::Owned(format!("{}", d)),
            LocalizedArg::VarDate(v) => v.get().map(|d| Cow::Owned(format!("{}", d))).unwrap_or_default(),
            LocalizedArg::ConstDateTime(dt) => Cow::Owned(format!("{}", dt)),
            LocalizedArg::VarDateTime(v) => v.get().map(|dt| Cow::Owned(format!("{}", dt))).unwrap_or_default(),
        }
    }
}

impl LocalizedArg {
    fn to_fluent<'a>(&'a self) -> Option<FluentValue<'a>> {
        match self {
            LocalizedArg::ConstString(s) => Some(FluentValue::String(Cow::Borrowed(s.as_ref()))),
            LocalizedArg::ConstNumber(n) => Some((*n).into()),
            LocalizedArg::VarString(v) => Some(v.get()?.into()),
            LocalizedArg::VarNumber(v) => Some(v.get()?.into()),
            #[cfg(feature = "icu")]
            LocalizedArg::ConstTime(_)
            | LocalizedArg::VarTime(_)
            | LocalizedArg::ConstDate(_)
            | LocalizedArg::VarDate(_)
            | LocalizedArg::ConstDateTime(_)
            | LocalizedArg::VarDateTime(_) => {
                match self {
                    LocalizedArg::VarTime(v) if !v.is_alive() => return None,
                    LocalizedArg::VarDate(v) if !v.is_alive() => return None,
                    LocalizedArg::VarDateTime(v) if !v.is_alive() => return None,
                    _ => {}
                }
                Some(FluentValue::Custom(Box::new(self.clone())))
            }
        }
    }

    fn get_key(&self) -> Option<VarKey> {
        match self {
            LocalizedArg::VarString(v) => Some(v.get_key()),
            LocalizedArg::VarNumber(v) => Some(v.get_key()),
            #[cfg(feature = "icu")]
            LocalizedArg::VarTime(v) => Some(v.get_key()),
            #[cfg(feature = "icu")]
            LocalizedArg::VarDate(v) => Some(v.get_key()),
            #[cfg(feature = "icu")]
            LocalizedArg::VarDateTime(v) => Some(v.get_key()),
            _ => None,
        }
    }

    fn get_version(&self) -> Option<u64> {
        match self {
            LocalizedArg::VarString(v) => v.get_version(),
            LocalizedArg::VarNumber(v) => v.get_version(),
            #[cfg(feature = "icu")]
            LocalizedArg::VarTime(v) => v.get_version(),
            #[cfg(feature = "icu")]
            LocalizedArg::VarDate(v) => v.get_version(),
            #[cfg(feature = "icu")]
            LocalizedArg::VarDateTime(v) => v.get_version(),
            _ => Some(0),
        }
    }
}

impl From<&'static str> for LocalizedArg {
    fn from(s: &'static str) -> Self {
        LocalizedArg::ConstString(Cow::Borrowed(s))
    }
}

impl From<String> for LocalizedArg {
    fn from(s: String) -> Self {
        LocalizedArg::ConstString(Cow::Owned(s))
    }
}

impl From<Cow<'static, str>> for LocalizedArg {
    fn from(c: Cow<'static, str>) -> Self {
        LocalizedArg::ConstString(c)
    }
}

impl From<f64> for LocalizedArg {
    fn from(n: f64) -> Self {
        LocalizedArg::ConstNumber(n)
    }
}

impl From<WeakVar<String>> for LocalizedArg {
    fn from(v: WeakVar<String>) -> Self {
        LocalizedArg::VarString(v)
    }
}

impl From<WeakVar<f64>> for LocalizedArg {
    fn from(v: WeakVar<f64>) -> Self {
        LocalizedArg::VarNumber(v)
    }
}

#[cfg(feature = "icu")]
impl From<Time> for LocalizedArg {
    fn from(t: Time) -> Self {
        LocalizedArg::ConstTime(t)
    }
}

#[cfg(feature = "icu")]
impl From<Date> for LocalizedArg {
    fn from(d: Date) -> Self {
        LocalizedArg::ConstDate(d)
    }
}

#[cfg(feature = "icu")]
impl From<OffsetDateTime> for LocalizedArg {
    fn from(dt: OffsetDateTime) -> Self {
        LocalizedArg::ConstDateTime(dt)
    }
}

#[cfg(feature = "icu")]
impl From<WeakVar<Time>> for LocalizedArg {
    fn from(v: WeakVar<Time>) -> Self {
        LocalizedArg::VarTime(v)
    }
}

#[cfg(feature = "icu")]
impl From<WeakVar<Date>> for LocalizedArg {
    fn from(v: WeakVar<Date>) -> Self {
        LocalizedArg::VarDate(v)
    }
}

#[cfg(feature = "icu")]
impl From<WeakVar<OffsetDateTime>> for LocalizedArg {
    fn from(v: WeakVar<OffsetDateTime>) -> Self {
        LocalizedArg::VarDateTime(v)
    }
}

/// Used to construct a [`LocalizedString`]
pub struct LocalizedStringBuilder {
    key: &'static str,
    placeholder: Option<&'static str>,
    args: Vec<(Cow<'static, str>, LocalizedArg)>,
}

impl LocalizedStringBuilder {
    /// Creates a new builder. `key` is used to look up translations in a [`TranslationFile`].
    pub fn new(key: &'static str) -> Self {
        Self {
            key,
            placeholder: None,
            args: Vec::new(),
        }
    }

    /// Provides a placeholder string to display when resolving fails.
    pub fn placeholder(mut self, text: &'static str) -> Self {
        self.placeholder = Some(text);
        self
    }

    /// Adds a value to be used as a localization argument when resolving the string.
    /// This can be a static value or a reactive variable.
    pub fn arg(mut self, key: &'static str, value: impl Into<LocalizedArg>) -> Self {
        self.args.push((Cow::Borrowed(key), value.into()));
        self
    }

    /// Builds the LocalizedString.
    pub fn build(self) -> LocalizedString {
        // VarKey contains a reference to mutable values
        // but the Hash and PartialEq impls use pointer values
        // so it's safe to use as a key in a hashmap.
        #[allow(clippy::mutable_key_type)]
        let mut dependencies = DependencyMap::default();
        for (_, arg) in &self.args {
            if let (Some(key), Some(version)) = (arg.get_key(), arg.get_version()) {
                dependencies.record(key, version);
            }
        }

        LocalizedString {
            inner: Arc::new(RwLock::new(LocalizedStringInner {
                last_loaded: None,
                key: Cow::Borrowed(self.key),
                placeholder: self.placeholder.map(Cow::Borrowed),
                args: self.args,
                last_locale: None,
                resolved_string: None,
            })),
        }
    }
}

struct LocalizedStringInner {
    last_loaded: Option<Instant>,
    key: Cow<'static, str>,
    placeholder: Option<Cow<'static, str>>,
    args: Vec<(Cow<'static, str>, LocalizedArg)>,
    last_locale: Option<LanguageIdentifier>,
    resolved_string: Option<String>,
}

impl fmt::Debug for LocalizedStringInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalizedString")
            .field("key", &self.key)
            .field("placeholder", &self.placeholder)
            .field("args", &self.args.len())
            .field("last_locale", &self.last_locale)
            .finish()
    }
}

/// Represents a string that can be localized into different languages.
///
/// Constructed with [`LocalizedStringBuilder`].
///
/// The contents are stored in an [`Arc<T>`] so it's cheap to clone.
#[derive(Clone)]
pub struct LocalizedString {
    inner: Arc<RwLock<LocalizedStringInner>>,
}

impl fmt::Debug for LocalizedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.read().fmt(f)
    }
}

impl LocalizedString {
    /// Resolves the string against the translation map
    pub fn resolve(&self, translation_map: &TranslationMap) -> MappedRwLockReadGuard<'_, str> {
        let map_guard = translation_map.inner.read();
        let current_locale = &map_guard.current_locale;

        let mut write_guard = self.inner.write();
        write_guard.resolved_string = None;
        write_guard.last_loaded = Some(map_guard.last_loaded);
        write_guard.last_locale = Some(current_locale.clone());

        // Attempt to resolve
        if let Some(file) = map_guard.translations.get(current_locale)
            && let Some(msg) = file.bundle.get_message(write_guard.key.as_ref())
            && let Some(value) = msg.value()
        {
            let mut args = FluentArgs::new();
            for (key, arg) in &write_guard.args {
                if let Some(fluent) = arg.to_fluent() {
                    args.set(key.as_ref(), fluent);
                }
            }

            let mut errors = Vec::new();
            let resolved = file.bundle.format_pattern(value, Some(&args), &mut errors).into_owned();

            if errors.is_empty() {
                write_guard.resolved_string = Some(resolved);
            } else {
                for error in errors {
                    error!("{error}");
                }
                // fallback to placeholder/key
                write_guard.resolved_string = None;
            }
        }

        let read_guard = RwLockWriteGuard::downgrade(write_guard);
        RwLockReadGuard::map(read_guard, |inner| {
            if let Some(text) = &inner.resolved_string {
                text.as_str()
            } else {
                inner.placeholder.as_deref().unwrap_or(inner.key.as_ref())
            }
        })
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct LocalizedStringSerde {
        key: Cow<'static, str>,
        placeholder: Option<Cow<'static, str>>,
        args: Vec<(Cow<'static, str>, LocalizedArg)>,
    }

    impl Serialize for LocalizedString {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let inner = self.inner.read();

            let ser = LocalizedStringSerde {
                key: inner.key.clone(),
                placeholder: inner.placeholder.clone(),
                args: inner.args.clone(),
            };

            ser.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for LocalizedString {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let de = LocalizedStringSerde::deserialize(deserializer)?;

            // VarKey contains a reference to mutable values
            // but the Hash and PartialEq impls use pointer values
            // so it's safe to use as a key in a hashmap.
            #[allow(clippy::mutable_key_type)]
            let mut dependencies = DependencyMap::default();
            for (_, arg) in &de.args {
                if let (Some(key), Some(version)) = (arg.get_key(), arg.get_version()) {
                    dependencies.record(key, version);
                }
            }

            Ok(LocalizedString {
                inner: Arc::new(RwLock::new(LocalizedStringInner {
                    last_loaded: None,
                    key: de.key,
                    placeholder: de.placeholder,
                    args: de.args,
                    last_locale: None,
                    resolved_string: None,
                })),
            })
        }
    }
}

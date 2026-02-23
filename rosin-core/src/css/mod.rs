//! Types related to styling the UI tree.
//!
//! This document assumes familiarity with CSS.
//!
//! ## Overview
//! Stylesheets are typically loaded with the [`stylesheet!()`](crate::stylesheet) macro,
//! but [`Stylesheet`] also implements [`std::str::FromStr`] and has a [`from_file`](crate::css::Stylesheet::from_file) method.
//!
//! In debug builds, stylesheets loaded with [`from_file`](crate::css::Stylesheet::from_file) will be automatically re-loaded when changed.
//!
//! Stylesheets are scoped, affecting only the nodes they're attached to and their descendants.
//! The [`Ui::style_sheet`](crate::tree::Ui::style_sheet) method is used to attach them to nodes.
//!
//! The supported properties are mostly standard CSS, with deviations primarily related to layout.
//! The [`layout`](crate::layout) module documentation explains what those properties do.
//!
//! `selection-background` and `selection-color` are the other non-standard properties available.
//! They control what text looks like when it's selected. Widgets must manually read those values in order to draw selected text correctly.
//!
//! Styles can also be changed dynamically at runtime by assigning an [`on_style`](crate::tree::Ui::on_style) callback to a node.
//!
//! ## Selectors
//! The following CSS selectors are supported:
//!
//! - Class selectors: `.button`, `.primary`, `.card`, etc.
//!   - Element selectors are treated as class selectors: `button { ... }` behaves like `.button { ... }`.
//!
//! - Wildcard selector: `*`
//!   - Matches any node.
//!
//! - Combinators
//!   - Descendant: `.panel .title { ... }`
//!     - Matches a `.title` with any `.panel` ancestors.
//!   - Child: `.panel > .title { ... }`
//!     - Matches `.title` only when it is an immediate child of `.panel`.
//!
//! - Pseudo-classes
//!   - `:hover`
//!   - `:focus`
//!   - `:active`
//!   - `:enabled`
//!   - `:disabled`
//!
//! - Grouping
//!   - Use commas to apply the same properties to multiple selectors: `.btn, .link { ... }`
//!
//! Specificity:
//!   - Standard CSS specificity rules apply for supported selectors.
//!   - If specificity ties, the rule that appears later in the stylesheet wins.
//!   - Rules from stylesheets higher in the tree always override rules from stylesheets lower in the tree.
//!
//! ## CSS Variables
//!
//! - Define variables with `--name: value;` inside any rule.
//! - Use them with `var(--name)` or `var(--name, fallback)`.
//!
//! ```css
//! .theme {
//!   --accent: lightblue;
//!   color: var(--accent);
//! }
//!
//! .danger {
//!   --accent: orange; /* overrides for this subtree */
//! }
//! ```
//!
//! ## Supported Properties
//!
//! All properties accept `initial | inherit`.
//!
//! `box-shadow` parses `inset` but the renderer doesn't support it yet, so inset shadows are currently ignored.
//!
//! **Grammar Notation:**
//!
//! ```text
//! A | B = alternatives
//! A B = sequence, space separated
//! [ A ] = optional
//! A? = optional (0 or 1)
//! A# = one or more, comma separated
//! A{1,4} = 1 to 4 occurrences
//! A || B || C = one or more, in any order
//! ```
//!
//! **Grammar Symbols:**
//!
//! ```text
//! <color> = a CSS <color> value (including currentColor).
//! <integer> = a CSS <integer> token.
//! <number> = a CSS <number> token.
//! <percentage> = <number>%
//! <length> = <number>px | <number>em | 0
//! <positive-length> = <length> (non-negative)
//! <unit> = auto | <percentage> | <length> | <stretch>
//! <positive-unit> = <unit> (non-negative)
//! <stretch> = <number> | <time>
//! <time> = <number>s | 0
//! <angle> =  0 | <number>deg | <number>rad | <number>grad | <number>turn
//! <angle-deg> = <number>deg
//!
//! <font-width> = <percentage> \| normal \| ultra-condensed \| extra-condensed
//!     \| condensed \| semi-condensed \| semi-expanded \| expanded
//!     \| extra-expanded \| ultra-expanded
//!
//! <stroke-shorthand> =
//!     [ ( <positive-length> | thin | medium | thick ) || solid || <color> ]
//!
//! <shadow> =
//!     [ inset ]? [ <color> ]? <length>{2,4}
//!
//! <text-shadow> =
//!     [ <color> ]? <length>{2,3}
//!
//! <translate-length> =
//!     0 | <number>px
//!
//! <transform-function> =
//!     translate(<translate-length> [ , ]? [ <translate-length> ]?)
//!     | rotate(<angle>)
//!     | scale(<number> [ , ]? [ <number> ]?)
//!     | skew(<angle> [ , ]? [ <angle> ]?)
//!     | matrix(<number>#{6})
//!
//! <side-or-corner> =
//!     left | right | top | bottom
//!     | left top | left bottom | right top | right bottom
//!
//! <color-space> =
//!     srgb | srgb-linear | linear-srgb | display-p3 | a98-rgb | prophoto-rgb
//!     | rec2020 | lab | lch | hsl | hwb | oklab | oklch | xyz-d50 | xyz | xyz-d65
//!     | acescg | aces-cg | aces2065-1
//!
//! <hue-direction> =
//!     shorter | longer | increasing | decreasing
//!
//! <stop-or-hint> =
//!     <color> [ <percentage> [ <percentage> ]? ]? | <percentage>
//!
//! <color-stop-list> =
//!     <stop-or-hint> ( , <stop-or-hint> )+
//!
//! <linear-gradient> =
//!     linear-gradient([ <angle> | to <side-or-corner> ]? [ , ]?
//!       [ in <color-space> [ <hue-direction> hue ]? ]? [ , ]?
//!       <color-stop-list>
//!     )
//!
//! <font> =
//!     [ <font-style> \|\| <font-weight> \|\| <font-width> ]?
//!     <font-size> [ / ( normal \| <unit> ) ]? <family-name>#
//! ```
//!
//! **Properties Table:**
//!
//! | Property | Value |
//! |---|---|
//! | `background-color` | `<color>` |
//! | `background-image` | `none \| <linear-gradient>#` |
//! | `border-bottom-color` | `<color>` |
//! | `border-bottom-left-radius` | `<positive-length>` |
//! | `border-bottom-right-radius` | `<positive-length>` |
//! | `border-bottom-width` | `<positive-length>` |
//! | `border-bottom` | `<stroke-shorthand>` |
//! | `border-color` | `<color>{1,4}` |
//! | `border-left-color` | `<color>` |
//! | `border-left-width` | `<positive-length>` |
//! | `border-left` | `<stroke-shorthand>` |
//! | `border-radius` | `<positive-length>{1,4}` |
//! | `border-right-color` | `<color>` |
//! | `border-right-width` | `<positive-length>` |
//! | `border-right` | `<stroke-shorthand>` |
//! | `border-top-color` | `<color>` |
//! | `border-top-left-radius` | `<positive-length>` |
//! | `border-top-right-radius` | `<positive-length>` |
//! | `border-top-width` | `<positive-length>` |
//! | `border-top` | `<stroke-shorthand>` |
//! | `border-width` | `( <positive-length> \| <line-width-keyword> ){1,4}` |
//! | `border` | `<stroke-shorthand>` |
//! | `bottom` | `<unit>` |
//! | `box-shadow` | `none \| <shadow>#` |
//! | `child-between` | `<positive-unit>` |
//! | `child-bottom` | `<positive-unit>` |
//! | `child-left` | `<positive-unit>` |
//! | `child-right` | `<positive-unit>` |
//! | `child-top` | `<positive-unit>` |
//! | `color` | `<color>` |
//! | `display` | `none \| row \| row-reverse \| column \| column-reverse` |
//! | `flex-basis` | `<positive-length>` |
//! | `font-family` | `<family-name>#` |
//! | `font-size` | `<number> \| <number>px` |
//! | `font-style` | `normal \| italic \| oblique [ <angle-deg> ]?` |
//! | `font-weight` | `normal \| bold \| <number>` |
//! | `font-width` | `<font-width>` |
//! | `font` | `<font>` |
//! | `height` | `<positive-unit>` |
//! | `left` | `<unit>` |
//! | `letter-spacing` | `<unit>` |
//! | `line-height` | `<positive-unit>` |
//! | `max-bottom` | `<positive-length>` |
//! | `max-child-between` | `<positive-length>` |
//! | `max-child-bottom` | `<positive-length>` |
//! | `max-child-left` | `<positive-length>` |
//! | `max-child-right` | `<positive-length>` |
//! | `max-child-top` | `<positive-length>` |
//! | `max-height` | `<positive-length>` |
//! | `max-left` | `<positive-length>` |
//! | `max-right` | `<positive-length>` |
//! | `max-top` | `<positive-length>` |
//! | `max-width` | `<positive-length>` |
//! | `min-bottom` | `<positive-length>` |
//! | `min-child-between` | `<positive-length>` |
//! | `min-child-bottom` | `<positive-length>` |
//! | `min-child-left` | `<positive-length>` |
//! | `min-child-right` | `<positive-length>` |
//! | `min-child-top` | `<positive-length>` |
//! | `min-height` | `<positive-length>` |
//! | `min-left` | `<positive-length>` |
//! | `min-right` | `<positive-length>` |
//! | `min-top` | `<positive-length>` |
//! | `min-width` | `<positive-length>` |
//! | `opacity` | `<number> \| <percentage>` |
//! | `outline-color` | `<color>` |
//! | `outline-offset` | `<length>` |
//! | `outline-width` | `<positive-length>` |
//! | `outline` | `<stroke-shorthand>` |
//! | `position` | `parent-directed \| self-directed \| fixed` |
//! | `right` | `<unit>` |
//! | `selection-background` | `<color>` |
//! | `selection-color` | `<color>` |
//! | `space` | `<unit>{1,4}` |
//! | `text-align` | `start \| end \| left \| right \| center \| justify` |
//! | `text-shadow` | `none \| <text-shadow>#` |
//! | `top` | `<unit>` |
//! | `transform` | `none \| <transform-function>+` |
//! | `width` | `<positive-unit>` |
//! | `word-spacing` | `<unit>` |
//! | `z-index` | `<integer>` |

mod parser;
mod properties;
mod style;
mod stylesheet;

pub use style::*;
pub use stylesheet::*;

pub(crate) const HOVER_DIRTY: u8 = 1 << 0;
pub(crate) const FOCUS_DIRTY: u8 = 1 << 1;
pub(crate) const ACTIVE_DIRTY: u8 = 1 << 2;
pub(crate) const ENABLED_DIRTY: u8 = 1 << 3;

#[inline]
pub(crate) fn log_error(msg: impl std::fmt::Display, location: cssparser::SourceLocation, file_name: Option<&std::path::Path>) {
    if let Some(path) = file_name {
        log::error!("{msg} {}:{}:{}", path.display(), location.line + 1, location.column);
    } else {
        log::error!("{msg} <no-filename>:{}:{}", location.line + 1, location.column);
    }
}

/// Loads a CSS [`Stylesheet`] from a path relative to the crate root.
///
/// In release builds, uses [`include_str`] to embed the CSS text into the binary.
#[macro_export]
macro_rules! stylesheet {
    ($path:literal) => {{
        #[cfg(not(debug_assertions))]
        {
            use std::str::FromStr;
            Stylesheet::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path))).expect("Failed to parse CSS")
        }

        #[cfg(debug_assertions)]
        {
            let css_path = concat!(env!("CARGO_MANIFEST_DIR"), "/", $path);
            Stylesheet::from_file(css_path).expect("Failed to parse CSS")
        }
    }};
}

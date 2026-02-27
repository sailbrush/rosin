use std::{
    error,
    fmt::{self, Display},
    sync::Arc,
};

use cssparser::{ParseErrorKind, Parser, ParserInput, SourceLocation, Token};
use kurbo::Affine;

use super::parser::*;
use super::*;
use crate::peniko::{
    self,
    color::{AlphaColor, Srgb},
};

const VAR_RESOLVE_LIMIT: usize = 8;

#[derive(Debug, Clone)]
pub(crate) enum VarResolveErrorKind {
    /// A `var(--x)` was encountered and `--x` was missing with no fallback.
    UnresolvedNoFallback,
    /// Too many iterative expansion passes.
    DepthExceeded,
    /// Parsing failed during variable expansion.
    ParseFailed,
}

#[derive(Debug, Clone)]
pub(crate) struct VarResolveError {
    pub kind: VarResolveErrorKind,
    pub value: Arc<str>,
    pub location: SourceLocation,
}

impl error::Error for VarResolveError {}
impl fmt::Display for VarResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            VarResolveErrorKind::UnresolvedNoFallback => {
                write!(f, "Unresolved var() reference (no fallback): `{}`", self.value)
            }
            VarResolveErrorKind::DepthExceeded => {
                write!(f, "var() expansion limit exceeded (possible cycle): `{}`", self.value)
            }
            VarResolveErrorKind::ParseFailed => {
                write!(f, "Invalid value after var() expansion: `{}`", self.value)
            }
        }
    }
}

/// An uninhabited type used to denote shorthand property values that don't have an Exact() value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoExact {}

impl Display for NoExact {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PropertyValue<T> {
    Initial,
    Inherit,
    Exact(T),
    Deferred(Arc<str>, SourceLocation),
}

impl<T: Display> Display for PropertyValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropertyValue::Initial => f.write_str("initial"),
            PropertyValue::Inherit => f.write_str("inherit"),
            PropertyValue::Exact(value) => value.fmt(f),
            PropertyValue::Deferred(string, _) => f.write_str(string),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ColorProperty {
    CurrentColor,
    Color(AlphaColor<Srgb>),
}

impl Eq for ColorProperty {}
impl PartialEq for ColorProperty {
    fn eq(&self, other: &Self) -> bool {
        use ColorProperty::*;

        match (self, other) {
            (CurrentColor, CurrentColor) => true,
            (Color(a), Color(b)) => a.to_rgba8() == b.to_rgba8(),
            _ => false,
        }
    }
}

impl ColorProperty {
    pub fn resolve(&self, current_color: AlphaColor<Srgb>) -> AlphaColor<Srgb> {
        match self {
            ColorProperty::CurrentColor => current_color,
            ColorProperty::Color(color) => *color,
        }
    }
}

impl Display for ColorProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorProperty::CurrentColor => f.write_str("currentcolor"),
            ColorProperty::Color(color) => {
                let color = color.to_rgba8();
                if color.a == 255 {
                    write!(f, "#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
                } else {
                    write!(f, "#{:02X}{:02X}{:02X}{:02X}", color.r, color.g, color.b, color.a)
                }
            }
        }
    }
}

impl From<AlphaColor<Srgb>> for ColorProperty {
    fn from(value: AlphaColor<Srgb>) -> Self {
        ColorProperty::Color(value)
    }
}

/// The result of parsing a CSS declaration.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Property {
    BackgroundColor(PropertyValue<ColorProperty>),
    BackgroundImage(PropertyValue<GradientStack>),
    Border(PropertyValue<NoExact>),
    BorderBottom(PropertyValue<NoExact>),
    BorderBottomColor(PropertyValue<ColorProperty>),
    BorderBottomLeftRadius(PropertyValue<Length>),
    BorderBottomRightRadius(PropertyValue<Length>),
    BorderBottomWidth(PropertyValue<Length>),
    BorderColor(PropertyValue<NoExact>),
    BorderLeft(PropertyValue<NoExact>),
    BorderLeftColor(PropertyValue<ColorProperty>),
    BorderLeftWidth(PropertyValue<Length>),
    BorderRadius(PropertyValue<NoExact>),
    BorderRight(PropertyValue<NoExact>),
    BorderRightColor(PropertyValue<ColorProperty>),
    BorderRightWidth(PropertyValue<Length>),
    BorderTop(PropertyValue<NoExact>),
    BorderTopColor(PropertyValue<ColorProperty>),
    BorderTopLeftRadius(PropertyValue<Length>),
    BorderTopRightRadius(PropertyValue<Length>),
    BorderTopWidth(PropertyValue<Length>),
    BorderWidth(PropertyValue<NoExact>),
    Bottom(PropertyValue<Unit>),
    BoxShadow(PropertyValue<Arc<[BoxShadow]>>),
    ChildBetween(PropertyValue<Unit>),
    ChildBottom(PropertyValue<Unit>),
    ChildLeft(PropertyValue<Unit>),
    ChildRight(PropertyValue<Unit>),
    ChildSpace(PropertyValue<NoExact>),
    ChildTop(PropertyValue<Unit>),
    Color(PropertyValue<ColorProperty>),
    Display(PropertyValue<Option<Direction>>),
    FlexBasis(PropertyValue<Length>),
    Font(PropertyValue<NoExact>),
    FontFamily(PropertyValue<Arc<str>>),
    FontSize(PropertyValue<f32>),
    FontStyle(PropertyValue<parley::style::FontStyle>),
    FontWeight(PropertyValue<f32>),
    FontWidth(PropertyValue<f32>),
    Height(PropertyValue<Unit>),
    Left(PropertyValue<Unit>),
    LetterSpacing(PropertyValue<Unit>),
    LineHeight(PropertyValue<Unit>),
    MaxBottom(PropertyValue<Length>),
    MaxChildBetween(PropertyValue<Length>),
    MaxChildBottom(PropertyValue<Length>),
    MaxChildLeft(PropertyValue<Length>),
    MaxChildRight(PropertyValue<Length>),
    MaxChildTop(PropertyValue<Length>),
    MaxHeight(PropertyValue<Length>),
    MaxLeft(PropertyValue<Length>),
    MaxRight(PropertyValue<Length>),
    MaxTop(PropertyValue<Length>),
    MaxWidth(PropertyValue<Length>),
    MinBottom(PropertyValue<Length>),
    MinChildBetween(PropertyValue<Length>),
    MinChildBottom(PropertyValue<Length>),
    MinChildLeft(PropertyValue<Length>),
    MinChildRight(PropertyValue<Length>),
    MinChildTop(PropertyValue<Length>),
    MinHeight(PropertyValue<Length>),
    MinLeft(PropertyValue<Length>),
    MinRight(PropertyValue<Length>),
    MinTop(PropertyValue<Length>),
    MinWidth(PropertyValue<Length>),
    Opacity(PropertyValue<f32>),
    Outline(PropertyValue<NoExact>),
    OutlineColor(PropertyValue<ColorProperty>),
    OutlineOffset(PropertyValue<Length>),
    OutlineWidth(PropertyValue<Length>),
    Position(PropertyValue<Position>),
    Right(PropertyValue<Unit>),
    SelectionBackground(PropertyValue<ColorProperty>),
    SelectionColor(PropertyValue<ColorProperty>),
    Space(PropertyValue<NoExact>),
    TextAlign(PropertyValue<TextAlign>),
    TextShadow(PropertyValue<Arc<[TextShadow]>>),
    Top(PropertyValue<Unit>),
    Transform(PropertyValue<Affine>),
    Width(PropertyValue<Unit>),
    WordSpacing(PropertyValue<Unit>),
    ZIndex(PropertyValue<i32>),
}

/// Reusable scratch space that [`Property::apply`] uses to avoid allocations.
#[derive(Default)]
pub(crate) struct ApplyScratch {
    one: String,
    two: String,
}

impl Property {
    /// Apply a property to a Style, resolving vars and re-parsing if needed.
    pub(crate) fn apply(
        &self,
        scratch: &mut ApplyScratch,
        style: &mut Style,
        parent_style: Option<&Style>,
        variables: &VariableContext,
    ) -> Result<(), VarResolveError> {
        if let Some((raw, location)) = self.deferred() {
            let resolved_opt = resolve_vars_iteratively(scratch, raw, location, variables)?;
            let value: &str = resolved_opt.unwrap_or_else(|| raw.as_ref());

            let props = self.reparse_value(value).map_err(|_| VarResolveError {
                kind: VarResolveErrorKind::ParseFailed,
                value: raw.clone(),
                location,
            })?;

            for p in props {
                p.apply_resolved(style, parent_style);
            }
            return Ok(());
        }

        self.apply_resolved(style, parent_style);
        Ok(())
    }
}

fn resolve_vars_iteratively<'a>(
    scratch: &'a mut ApplyScratch,
    raw: &Arc<str>,
    location: SourceLocation,
    vars: &VariableContext,
) -> Result<Option<&'a str>, VarResolveError> {
    let input: &str = raw.as_ref();

    scratch.one.clear();
    scratch.two.clear();

    // First pass writes into one
    if !resolve_vars_pass(input, raw, location, vars, &mut scratch.one)? {
        return Ok(None);
    }

    for _ in 1..VAR_RESOLVE_LIMIT {
        // Next pass writes into two
        if !resolve_vars_pass(&scratch.one, raw, location, vars, &mut scratch.two)? {
            return Ok(Some(scratch.one.as_str()));
        }

        std::mem::swap(&mut scratch.one, &mut scratch.two);
        scratch.two.clear();
    }

    Err(VarResolveError {
        kind: VarResolveErrorKind::DepthExceeded,
        value: raw.clone(),
        location,
    })
}

/// Returns true if at least one var() was found and expanded into `out`
fn resolve_vars_pass(input: &str, raw: &Arc<str>, location: SourceLocation, vars: &VariableContext, out: &mut String) -> Result<bool, VarResolveError> {
    enum Replace<'i, 'a> {
        Value(&'a str),
        /// Borrowed slice of `input` that represents the fallback content (after the comma)
        Fallback(&'i str),
    }

    struct Ctx<'i, 'a> {
        input: &'i str,
        raw: &'a Arc<str>,
        location: SourceLocation,
        vars: &'a VariableContext,
        out: &'a mut String,
        wrote: bool,
        orig_len: usize,
        last_flush: usize,
    }

    impl<'i, 'a> Ctx<'i, 'a> {
        /// Consume the rest of the current parser's input, properly recursing into nested blocks
        fn consume_to_end(&mut self, p: &mut Parser<'i, '_>) -> Result<(), cssparser::ParseError<'i, VarResolveError>> {
            while !p.is_exhausted() {
                let tok = p.next_including_whitespace_and_comments()?;
                match tok {
                    Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                        p.parse_nested_block(|p| self.consume_to_end(p))?;
                    }
                    // Reject stray/unbalanced delimiters in the component-value stream
                    Token::Delim('(')
                    | Token::Delim('[')
                    | Token::Delim('{')
                    | Token::CloseParenthesis
                    | Token::CloseSquareBracket
                    | Token::CloseCurlyBracket => {
                        return Err(p.new_custom_error({
                            VarResolveError {
                                kind: VarResolveErrorKind::ParseFailed,
                                value: self.raw.clone(),
                                location: self.location,
                            }
                        }));
                    }
                    _ => {}
                }
            }
            Ok(())
        }

        fn walk(&mut self, p: &mut Parser<'i, '_>) -> Result<(), cssparser::ParseError<'i, VarResolveError>> {
            while !p.is_exhausted() {
                let tok_start = p.position().byte_index();

                match p.next_including_whitespace_and_comments()? {
                    Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                        // Decide what to splice in, and advance parser past the var() content
                        let rep: Replace<'i, 'a> = p.parse_nested_block(|p| {
                            p.skip_whitespace();

                            let value = {
                                let ident = p.expect_ident()?;
                                self.vars.get(ident.as_ref())
                            };

                            p.skip_whitespace();
                            let has_comma = p.try_parse(|p| p.expect_comma()).is_ok();

                            if let Some(v) = value {
                                // Consume optional fallback (if present), recursing properly
                                if has_comma {
                                    self.consume_to_end(p)?;
                                } else {
                                    p.skip_whitespace();
                                }
                                Ok(Replace::Value(v))
                            } else if has_comma {
                                // Capture fallback slice after the comma, but consume with recursion
                                p.skip_whitespace();
                                let fb_start_pos = p.position();
                                self.consume_to_end(p)?;
                                let fb = p.slice_from(fb_start_pos);
                                Ok(Replace::Fallback(fb))
                            } else {
                                Err(p.new_custom_error(VarResolveError {
                                    kind: VarResolveErrorKind::UnresolvedNoFallback,
                                    value: self.raw.clone(),
                                    location: self.location,
                                }))
                            }
                        })?;

                        // Lazily start writing on the first change
                        if !self.wrote {
                            self.out.truncate(self.orig_len);
                            self.out.reserve(self.input.len());
                            self.out.push_str(&self.input[..tok_start]);
                            self.wrote = true;
                        } else {
                            self.out.push_str(&self.input[self.last_flush..tok_start]);
                        }

                        match rep {
                            Replace::Value(v) => self.out.push_str(v),
                            Replace::Fallback(fb) => self.out.push_str(fb),
                        }

                        // After parse_nested_block, we're positioned just after the closing ')'
                        self.last_flush = p.position().byte_index();
                    }
                    // Recurse into nested blocks so we can resolve var() inside them
                    Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                        p.parse_nested_block(|p| self.walk(p))?;
                    }

                    // Reject stray/unbalanced delimiters at top level too
                    Token::Delim('(')
                    | Token::Delim('[')
                    | Token::Delim('{')
                    | Token::CloseParenthesis
                    | Token::CloseSquareBracket
                    | Token::CloseCurlyBracket => {
                        return Err(p.new_custom_error({
                            VarResolveError {
                                kind: VarResolveErrorKind::ParseFailed,
                                value: self.raw.clone(),
                                location: self.location,
                            }
                        }));
                    }

                    _ => {}
                }
            }

            Ok(())
        }
    }

    let orig_len = out.len();

    let mut ctx = Ctx {
        input,
        raw,
        location,
        vars,
        out,
        wrote: false,
        orig_len,
        last_flush: 0,
    };

    let mut inpt = ParserInput::new(input);
    let mut p = Parser::new(&mut inpt);

    ctx.walk(&mut p).map_err(|e| {
        if let ParseErrorKind::Custom(v) = e.kind {
            v
        } else {
            VarResolveError {
                kind: VarResolveErrorKind::ParseFailed,
                value: raw.clone(),
                location,
            }
        }
    })?;

    if !ctx.wrote {
        return Ok(false);
    }

    // Flush the remaining text
    ctx.out.push_str(&ctx.input[ctx.last_flush..]);
    Ok(true)
}

fn apply_value<T: Clone>(value: &PropertyValue<T>, out: &mut T, parent_style: Option<&Style>, def: &Style, get: fn(&Style) -> &T) {
    let default = get(def).clone();
    match value {
        PropertyValue::Exact(v) => *out = v.clone(),
        PropertyValue::Inherit => *out = parent_style.map(get).cloned().unwrap_or(default),
        PropertyValue::Initial => *out = default,
        PropertyValue::Deferred(..) => {}
    }
}

fn apply_opt<T: Clone>(v: &PropertyValue<T>, out: &mut Option<T>, parent_style: Option<&Style>, default: &Style, get: fn(&Style) -> &Option<T>) {
    let def = get(default).clone();
    match v {
        PropertyValue::Exact(v) => *out = Some(v.clone()),
        PropertyValue::Inherit => *out = parent_style.map(get).cloned().unwrap_or(def),
        PropertyValue::Initial => *out = def,
        PropertyValue::Deferred(..) => {}
    }
}

fn apply_color(
    value: &PropertyValue<ColorProperty>,
    out: &mut peniko::Color,
    parent_style: Option<&Style>,
    default: &Style,
    current: peniko::Color,
    get: fn(&Style) -> &peniko::Color,
) {
    let def = *get(default);
    match value {
        PropertyValue::Exact(v) => *out = v.resolve(current),
        PropertyValue::Inherit => *out = parent_style.map(get).cloned().unwrap_or(def),
        PropertyValue::Initial => *out = def,
        PropertyValue::Deferred(..) => {}
    }
}

fn apply_opt_color(
    value: &PropertyValue<ColorProperty>,
    out: &mut Option<peniko::Color>,
    parent_style: Option<&Style>,
    default: &Style,
    current: peniko::Color,
    get: fn(&Style) -> &Option<peniko::Color>,
) {
    let def = *get(default);
    match value {
        PropertyValue::Exact(v) => *out = Some(v.resolve(current)),
        PropertyValue::Inherit => *out = parent_style.map(get).cloned().unwrap_or(def),
        PropertyValue::Initial => *out = def,
        PropertyValue::Deferred(..) => {}
    }
}

fn apply_opt_unit(value: &PropertyValue<Unit>, out: &mut Option<Unit>, parent_style: Option<&Style>, default: &Style, get: fn(&Style) -> &Option<Unit>) {
    let def = *get(default);
    *out = match value {
        PropertyValue::Exact(Unit::Auto) => None,
        PropertyValue::Exact(u) => Some(*u),
        PropertyValue::Inherit => parent_style.map(get).cloned().unwrap_or(def),
        PropertyValue::Initial => def,
        PropertyValue::Deferred(..) => return,
    };
}

fn fmt_comma_separated_property<T: Display>(name: &str, value: &PropertyValue<Arc<[T]>>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{name}")?;
    match value {
        PropertyValue::Initial => f.write_str("initial"),
        PropertyValue::Inherit => f.write_str("inherit"),
        PropertyValue::Exact(array) => {
            for (i, item) in array.iter().enumerate() {
                if i != 0 {
                    f.write_str(",")?;
                }
                write!(f, "{item}")?;
            }
            Ok(())
        }
        PropertyValue::Deferred(string, _) => f.write_str(string),
    }?;
    write!(f, ";")
}

fn fmt_display_property(value: &PropertyValue<Option<Direction>>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "display:")?;
    match value {
        PropertyValue::Initial => f.write_str("initial"),
        PropertyValue::Inherit => f.write_str("inherit"),
        PropertyValue::Exact(None) => f.write_str("none"),
        PropertyValue::Exact(Some(display)) => write!(f, "{display}"),
        PropertyValue::Deferred(string, _) => f.write_str(string),
    }?;
    write!(f, ";")
}

fn fmt_transform_property(value: &PropertyValue<Affine>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "transform:")?;
    match value {
        PropertyValue::Initial => f.write_str("initial"),
        PropertyValue::Inherit => f.write_str("inherit"),
        PropertyValue::Deferred(string, _) => f.write_str(string),
        PropertyValue::Exact(aff) => {
            if *aff == Affine::IDENTITY {
                return f.write_str("none");
            }

            let [a, b, c, d, e, ff] = aff.as_coeffs();
            write!(f, "matrix({a},{b},{c},{d},{e},{ff})")
        }
    }?;
    write!(f, ";")
}

impl Display for Property {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Property::BoxShadow(value) => fmt_comma_separated_property("box-shadow:", value, f),
            Property::TextShadow(value) => fmt_comma_separated_property("text-shadow:", value, f),
            Property::Display(value) => fmt_display_property(value, f),
            Property::BackgroundColor(value) => write!(f, "background-color:{value};"),
            Property::BackgroundImage(value) => write!(f, "background-image:{value};"),
            Property::Border(value) => write!(f, "border:{:?};", value),
            Property::BorderBottom(value) => write!(f, "border-bottom:{:?};", value),
            Property::BorderBottomColor(value) => write!(f, "border-bottom-color:{value};"),
            Property::BorderBottomLeftRadius(value) => {
                write!(f, "border-bottom-left-radius:{value};")
            }
            Property::BorderBottomRightRadius(value) => {
                write!(f, "border-bottom-right-radius:{value};")
            }
            Property::BorderBottomWidth(value) => write!(f, "border-bottom-width:{value};"),
            Property::BorderColor(value) => write!(f, "border-color:{:?};", value),
            Property::BorderLeft(value) => write!(f, "border-left:{:?};", value),
            Property::BorderLeftColor(value) => write!(f, "border-left-color:{value};"),
            Property::BorderLeftWidth(value) => write!(f, "border-left-width:{value};"),
            Property::BorderRadius(value) => write!(f, "border-radius:{:?};", value),
            Property::BorderRight(value) => write!(f, "border-right:{:?};", value),
            Property::BorderRightColor(value) => write!(f, "border-right-color:{value};"),
            Property::BorderRightWidth(value) => write!(f, "border-right-width:{value};"),
            Property::BorderTop(value) => write!(f, "border-top:{:?};", value),
            Property::BorderTopColor(value) => write!(f, "border-top-color:{value};"),
            Property::BorderTopLeftRadius(value) => write!(f, "border-top-left-radius:{value};"),
            Property::BorderTopRightRadius(value) => write!(f, "border-top-right-radius:{value};"),
            Property::BorderTopWidth(value) => write!(f, "border-top-width:{value};"),
            Property::BorderWidth(value) => write!(f, "border-width:{:?};", value),
            Property::Bottom(value) => write!(f, "bottom:{value};"),
            Property::ChildBetween(value) => write!(f, "child-between:{value};"),
            Property::ChildBottom(value) => write!(f, "child-bottom:{value};"),
            Property::ChildLeft(value) => write!(f, "child-left:{value};"),
            Property::ChildRight(value) => write!(f, "child-right:{value};"),
            Property::ChildTop(value) => write!(f, "child-top:{value};"),
            Property::Color(value) => write!(f, "color:{value};"),
            Property::FlexBasis(value) => write!(f, "flex-basis:{value};"),
            Property::Font(value) => write!(f, "font:{:?};", value),
            Property::FontFamily(value) => write!(f, "font-family:{value};"),
            Property::FontSize(value) => write!(f, "font-size:{value};"),
            Property::FontStyle(value) => write!(f, "font-style:{value};"),
            Property::FontWeight(value) => write!(f, "font-weight:{value};"),
            Property::FontWidth(value) => write!(f, "font-width:{value};"),
            Property::Height(value) => write!(f, "height:{value};"),
            Property::Left(value) => write!(f, "left:{value};"),
            Property::LetterSpacing(value) => write!(f, "letter-spacing:{value};"),
            Property::LineHeight(value) => write!(f, "line-height:{value};"),
            Property::MaxBottom(value) => write!(f, "max-bottom:{value};"),
            Property::MaxChildBetween(value) => write!(f, "max-child-between:{value};"),
            Property::MaxChildBottom(value) => write!(f, "max-child-bottom:{value};"),
            Property::MaxChildLeft(value) => write!(f, "max-child-left:{value};"),
            Property::MaxChildRight(value) => write!(f, "max-child-right:{value};"),
            Property::MaxChildTop(value) => write!(f, "max-child-top:{value};"),
            Property::MaxHeight(value) => write!(f, "max-height:{value};"),
            Property::MaxLeft(value) => write!(f, "max-left:{value};"),
            Property::MaxRight(value) => write!(f, "max-right:{value};"),
            Property::MaxTop(value) => write!(f, "max-top:{value};"),
            Property::MaxWidth(value) => write!(f, "max-width:{value};"),
            Property::MinBottom(value) => write!(f, "min-bottom:{value};"),
            Property::MinChildBetween(value) => write!(f, "min-child-between:{value};"),
            Property::MinChildBottom(value) => write!(f, "min-child-bottom:{value};"),
            Property::MinChildLeft(value) => write!(f, "min-child-left:{value};"),
            Property::MinChildRight(value) => write!(f, "min-child-right:{value};"),
            Property::MinChildTop(value) => write!(f, "min-child-top:{value};"),
            Property::MinHeight(value) => write!(f, "min-height:{value};"),
            Property::MinLeft(value) => write!(f, "min-left:{value};"),
            Property::MinRight(value) => write!(f, "min-right:{value};"),
            Property::MinTop(value) => write!(f, "min-top:{value};"),
            Property::MinWidth(value) => write!(f, "min-width:{value};"),
            Property::Opacity(value) => write!(f, "opacity:{value};"),
            Property::Outline(value) => write!(f, "outline:{:?};", value),
            Property::OutlineColor(value) => write!(f, "outline-color:{value};"),
            Property::OutlineOffset(value) => write!(f, "outline-offset:{value};"),
            Property::OutlineWidth(value) => write!(f, "outline-width:{value};"),
            Property::Position(value) => write!(f, "position:{value};"),
            Property::Right(value) => write!(f, "right:{value};"),
            Property::SelectionBackground(value) => write!(f, "selection-background:{value};"),
            Property::SelectionColor(value) => write!(f, "selection-color:{value};"),
            Property::Space(value) => write!(f, "space:{:?};", value),
            Property::ChildSpace(value) => write!(f, "child-space:{:?};", value),
            Property::TextAlign(value) => write!(f, "text-align:{value};"),
            Property::Transform(value) => fmt_transform_property(value, f),
            Property::Top(value) => write!(f, "top:{value};"),
            Property::Width(value) => write!(f, "width:{value};"),
            Property::WordSpacing(value) => write!(f, "word-spacing:{value};"),
            Property::ZIndex(value) => write!(f, "z-index:{value};"),
        }
    }
}

impl Property {
    pub const fn affects_layout(&self) -> bool {
        // This needs to match the values in style::LayoutStyle
        match self {
            Property::BackgroundColor(_) => false,
            Property::BackgroundImage(_) => false,
            Property::Border(_) => true,
            Property::BorderBottom(_) => true,
            Property::BorderBottomColor(_) => false,
            Property::BorderBottomLeftRadius(_) => true,
            Property::BorderBottomRightRadius(_) => true,
            Property::BorderBottomWidth(_) => true,
            Property::BorderColor(_) => false,
            Property::BorderLeft(_) => true,
            Property::BorderLeftColor(_) => false,
            Property::BorderLeftWidth(_) => true,
            Property::BorderRadius(_) => true,
            Property::BorderRight(_) => true,
            Property::BorderRightColor(_) => false,
            Property::BorderRightWidth(_) => true,
            Property::BorderTop(_) => true,
            Property::BorderTopColor(_) => false,
            Property::BorderTopLeftRadius(_) => true,
            Property::BorderTopRightRadius(_) => true,
            Property::BorderTopWidth(_) => true,
            Property::BorderWidth(_) => true,
            Property::Bottom(_) => true,
            Property::BoxShadow(_) => false,
            Property::ChildBetween(_) => true,
            Property::ChildBottom(_) => true,
            Property::ChildLeft(_) => true,
            Property::ChildRight(_) => true,
            Property::ChildSpace(_) => true,
            Property::ChildTop(_) => true,
            Property::Color(_) => false,
            Property::Display(_) => true,
            Property::FlexBasis(_) => true,
            Property::Font(_) => true,
            Property::FontFamily(_) => true,
            Property::FontSize(_) => true,
            Property::FontStyle(_) => true,
            Property::FontWeight(_) => true,
            Property::FontWidth(_) => true,
            Property::Height(_) => true,
            Property::Left(_) => true,
            Property::LetterSpacing(_) => true,
            Property::LineHeight(_) => true,
            Property::MaxBottom(_) => true,
            Property::MaxChildBetween(_) => true,
            Property::MaxChildBottom(_) => true,
            Property::MaxChildLeft(_) => true,
            Property::MaxChildRight(_) => true,
            Property::MaxChildTop(_) => true,
            Property::MaxHeight(_) => true,
            Property::MaxLeft(_) => true,
            Property::MaxRight(_) => true,
            Property::MaxTop(_) => true,
            Property::MaxWidth(_) => true,
            Property::MinBottom(_) => true,
            Property::MinChildBetween(_) => true,
            Property::MinChildBottom(_) => true,
            Property::MinChildLeft(_) => true,
            Property::MinChildRight(_) => true,
            Property::MinChildTop(_) => true,
            Property::MinHeight(_) => true,
            Property::MinLeft(_) => true,
            Property::MinRight(_) => true,
            Property::MinTop(_) => true,
            Property::MinWidth(_) => true,
            Property::Opacity(_) => false,
            Property::Outline(_) => false,
            Property::OutlineColor(_) => false,
            Property::OutlineOffset(_) => false,
            Property::OutlineWidth(_) => false,
            Property::Position(_) => true,
            Property::Right(_) => true,
            Property::SelectionBackground(_) => false,
            Property::SelectionColor(_) => false,
            Property::Space(_) => true,
            Property::TextAlign(_) => true,
            Property::TextShadow(_) => false,
            Property::Top(_) => true,
            Property::Transform(_) => false,
            Property::Width(_) => true,
            Property::WordSpacing(_) => true,
            Property::ZIndex(_) => false,
        }
    }

    fn deferred(&self) -> Option<(&Arc<str>, SourceLocation)> {
        use PropertyValue::Deferred;

        match self {
            Property::BackgroundColor(Deferred(s, loc)) => Some((s, *loc)),
            Property::BackgroundImage(Deferred(s, loc)) => Some((s, *loc)),
            Property::Border(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderBottom(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderBottomColor(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderBottomLeftRadius(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderBottomRightRadius(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderBottomWidth(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderColor(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderLeft(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderLeftColor(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderLeftWidth(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderRadius(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderRight(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderRightColor(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderRightWidth(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderTop(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderTopColor(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderTopLeftRadius(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderTopRightRadius(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderTopWidth(Deferred(s, loc)) => Some((s, *loc)),
            Property::BorderWidth(Deferred(s, loc)) => Some((s, *loc)),
            Property::Bottom(Deferred(s, loc)) => Some((s, *loc)),
            Property::BoxShadow(Deferred(s, loc)) => Some((s, *loc)),
            Property::ChildBetween(Deferred(s, loc)) => Some((s, *loc)),
            Property::ChildBottom(Deferred(s, loc)) => Some((s, *loc)),
            Property::ChildLeft(Deferred(s, loc)) => Some((s, *loc)),
            Property::ChildRight(Deferred(s, loc)) => Some((s, *loc)),
            Property::ChildSpace(Deferred(s, loc)) => Some((s, *loc)),
            Property::ChildTop(Deferred(s, loc)) => Some((s, *loc)),
            Property::Color(Deferred(s, loc)) => Some((s, *loc)),
            Property::Display(Deferred(s, loc)) => Some((s, *loc)),
            Property::FlexBasis(Deferred(s, loc)) => Some((s, *loc)),
            Property::Font(Deferred(s, loc)) => Some((s, *loc)),
            Property::FontFamily(Deferred(s, loc)) => Some((s, *loc)),
            Property::FontSize(Deferred(s, loc)) => Some((s, *loc)),
            Property::FontStyle(Deferred(s, loc)) => Some((s, *loc)),
            Property::FontWeight(Deferred(s, loc)) => Some((s, *loc)),
            Property::FontWidth(Deferred(s, loc)) => Some((s, *loc)),
            Property::Height(Deferred(s, loc)) => Some((s, *loc)),
            Property::Left(Deferred(s, loc)) => Some((s, *loc)),
            Property::LetterSpacing(Deferred(s, loc)) => Some((s, *loc)),
            Property::LineHeight(Deferred(s, loc)) => Some((s, *loc)),
            Property::MaxBottom(Deferred(s, loc)) => Some((s, *loc)),
            Property::MaxChildBetween(Deferred(s, loc)) => Some((s, *loc)),
            Property::MaxChildBottom(Deferred(s, loc)) => Some((s, *loc)),
            Property::MaxChildLeft(Deferred(s, loc)) => Some((s, *loc)),
            Property::MaxChildRight(Deferred(s, loc)) => Some((s, *loc)),
            Property::MaxChildTop(Deferred(s, loc)) => Some((s, *loc)),
            Property::MaxHeight(Deferred(s, loc)) => Some((s, *loc)),
            Property::MaxLeft(Deferred(s, loc)) => Some((s, *loc)),
            Property::MaxRight(Deferred(s, loc)) => Some((s, *loc)),
            Property::MaxTop(Deferred(s, loc)) => Some((s, *loc)),
            Property::MaxWidth(Deferred(s, loc)) => Some((s, *loc)),
            Property::MinBottom(Deferred(s, loc)) => Some((s, *loc)),
            Property::MinChildBetween(Deferred(s, loc)) => Some((s, *loc)),
            Property::MinChildBottom(Deferred(s, loc)) => Some((s, *loc)),
            Property::MinChildLeft(Deferred(s, loc)) => Some((s, *loc)),
            Property::MinChildRight(Deferred(s, loc)) => Some((s, *loc)),
            Property::MinChildTop(Deferred(s, loc)) => Some((s, *loc)),
            Property::MinHeight(Deferred(s, loc)) => Some((s, *loc)),
            Property::MinLeft(Deferred(s, loc)) => Some((s, *loc)),
            Property::MinRight(Deferred(s, loc)) => Some((s, *loc)),
            Property::MinTop(Deferred(s, loc)) => Some((s, *loc)),
            Property::MinWidth(Deferred(s, loc)) => Some((s, *loc)),
            Property::Opacity(Deferred(s, loc)) => Some((s, *loc)),
            Property::Outline(Deferred(s, loc)) => Some((s, *loc)),
            Property::OutlineColor(Deferred(s, loc)) => Some((s, *loc)),
            Property::OutlineOffset(Deferred(s, loc)) => Some((s, *loc)),
            Property::OutlineWidth(Deferred(s, loc)) => Some((s, *loc)),
            Property::Position(Deferred(s, loc)) => Some((s, *loc)),
            Property::Right(Deferred(s, loc)) => Some((s, *loc)),
            Property::SelectionBackground(Deferred(s, loc)) => Some((s, *loc)),
            Property::SelectionColor(Deferred(s, loc)) => Some((s, *loc)),
            Property::Space(Deferred(s, loc)) => Some((s, *loc)),
            Property::TextAlign(Deferred(s, loc)) => Some((s, *loc)),
            Property::TextShadow(Deferred(s, loc)) => Some((s, *loc)),
            Property::Top(Deferred(s, loc)) => Some((s, *loc)),
            Property::Transform(Deferred(s, loc)) => Some((s, *loc)),
            Property::Width(Deferred(s, loc)) => Some((s, *loc)),
            Property::WordSpacing(Deferred(s, loc)) => Some((s, *loc)),
            Property::ZIndex(Deferred(s, loc)) => Some((s, *loc)),
            _ => None,
        }
    }

    fn reparse_value<'i>(&self, value: &'i str) -> Result<Props, cssparser::ParseError<'i, CustomParseError>> {
        let mut input = ParserInput::new(value);
        let mut parser = Parser::new(&mut input);

        match self {
            Property::BackgroundColor(_) => parse_property(&mut parser, parse_color, Property::BackgroundColor),
            Property::BackgroundImage(_) => parse_property(&mut parser, parse_background_image, Property::BackgroundImage),
            Property::Border(_) => parse_shorthand(&mut parser, parse_border_sh, Property::Border),
            Property::BorderBottom(_) => parse_shorthand(&mut parser, |p| parse_border_side_sh(p, BorderSide::Bottom), Property::BorderBottom),
            Property::BorderBottomColor(_) => parse_property(&mut parser, parse_color, Property::BorderBottomColor),
            Property::BorderBottomLeftRadius(_) => parse_property(&mut parser, parse_positive_length, Property::BorderBottomLeftRadius),
            Property::BorderBottomRightRadius(_) => parse_property(&mut parser, parse_positive_length, Property::BorderBottomRightRadius),
            Property::BorderBottomWidth(_) => parse_property(&mut parser, parse_positive_length, Property::BorderBottomWidth),
            Property::BorderColor(_) => parse_shorthand(&mut parser, parse_border_color_sh, Property::BorderColor),
            Property::BorderLeft(_) => parse_shorthand(&mut parser, |p| parse_border_side_sh(p, BorderSide::Left), Property::BorderLeft),
            Property::BorderLeftColor(_) => parse_property(&mut parser, parse_color, Property::BorderLeftColor),
            Property::BorderLeftWidth(_) => parse_property(&mut parser, parse_positive_length, Property::BorderLeftWidth),
            Property::BorderRadius(_) => parse_shorthand(&mut parser, parse_border_radius_sh, Property::BorderRadius),
            Property::BorderRight(_) => parse_shorthand(&mut parser, |p| parse_border_side_sh(p, BorderSide::Right), Property::BorderRight),
            Property::BorderRightColor(_) => parse_property(&mut parser, parse_color, Property::BorderRightColor),
            Property::BorderRightWidth(_) => parse_property(&mut parser, parse_positive_length, Property::BorderRightWidth),
            Property::BorderTop(_) => parse_shorthand(&mut parser, |p| parse_border_side_sh(p, BorderSide::Top), Property::BorderTop),
            Property::BorderTopColor(_) => parse_property(&mut parser, parse_color, Property::BorderTopColor),
            Property::BorderTopLeftRadius(_) => parse_property(&mut parser, parse_positive_length, Property::BorderTopLeftRadius),
            Property::BorderTopRightRadius(_) => parse_property(&mut parser, parse_positive_length, Property::BorderTopRightRadius),
            Property::BorderTopWidth(_) => parse_property(&mut parser, parse_positive_length, Property::BorderTopWidth),
            Property::BorderWidth(_) => parse_shorthand(&mut parser, parse_border_width_sh, Property::BorderWidth),
            Property::Bottom(_) => parse_property(&mut parser, parse_unit, Property::Bottom),
            Property::BoxShadow(_) => parse_property(&mut parser, parse_box_shadow, Property::BoxShadow),
            Property::ChildBetween(_) => parse_property(&mut parser, parse_positive_unit, Property::ChildBetween),
            Property::ChildBottom(_) => parse_property(&mut parser, parse_positive_unit, Property::ChildBottom),
            Property::ChildLeft(_) => parse_property(&mut parser, parse_positive_unit, Property::ChildLeft),
            Property::ChildRight(_) => parse_property(&mut parser, parse_positive_unit, Property::ChildRight),
            Property::ChildSpace(_) => parse_shorthand(&mut parser, parse_child_space_sh, Property::ChildSpace),
            Property::ChildTop(_) => parse_property(&mut parser, parse_positive_unit, Property::ChildTop),
            Property::Color(_) => parse_property(&mut parser, parse_color, Property::Color),
            Property::Display(_) => parse_property(&mut parser, parse_display, Property::Display),
            Property::FlexBasis(_) => parse_property(&mut parser, parse_positive_length, Property::FlexBasis),
            Property::Font(_) => parse_shorthand(&mut parser, parse_font_sh, Property::Font),
            Property::FontFamily(_) => parse_property(&mut parser, parse_font_family, Property::FontFamily),
            Property::FontSize(_) => parse_property(&mut parser, parse_font_size, Property::FontSize),
            Property::FontStyle(_) => parse_property(&mut parser, parse_font_style, Property::FontStyle),
            Property::FontWeight(_) => parse_property(&mut parser, parse_font_weight, Property::FontWeight),
            Property::FontWidth(_) => parse_property(&mut parser, parse_font_width, Property::FontWidth),
            Property::Height(_) => parse_property(&mut parser, parse_positive_unit, Property::Height),
            Property::Left(_) => parse_property(&mut parser, parse_unit, Property::Left),
            Property::LetterSpacing(_) => parse_property(&mut parser, parse_unit, Property::LetterSpacing),
            Property::LineHeight(_) => parse_property(&mut parser, parse_positive_unit, Property::LineHeight),
            Property::MaxBottom(_) => parse_property(&mut parser, parse_positive_length, Property::MaxBottom),
            Property::MaxChildBetween(_) => parse_property(&mut parser, parse_positive_length, Property::MaxChildBetween),
            Property::MaxChildBottom(_) => parse_property(&mut parser, parse_positive_length, Property::MaxChildBottom),
            Property::MaxChildLeft(_) => parse_property(&mut parser, parse_positive_length, Property::MaxChildLeft),
            Property::MaxChildRight(_) => parse_property(&mut parser, parse_positive_length, Property::MaxChildRight),
            Property::MaxChildTop(_) => parse_property(&mut parser, parse_positive_length, Property::MaxChildTop),
            Property::MaxHeight(_) => parse_property(&mut parser, parse_positive_length, Property::MaxHeight),
            Property::MaxLeft(_) => parse_property(&mut parser, parse_positive_length, Property::MaxLeft),
            Property::MaxRight(_) => parse_property(&mut parser, parse_positive_length, Property::MaxRight),
            Property::MaxTop(_) => parse_property(&mut parser, parse_positive_length, Property::MaxTop),
            Property::MaxWidth(_) => parse_property(&mut parser, parse_positive_length, Property::MaxWidth),
            Property::MinBottom(_) => parse_property(&mut parser, parse_positive_length, Property::MinBottom),
            Property::MinChildBetween(_) => parse_property(&mut parser, parse_positive_length, Property::MinChildBetween),
            Property::MinChildBottom(_) => parse_property(&mut parser, parse_positive_length, Property::MinChildBottom),
            Property::MinChildLeft(_) => parse_property(&mut parser, parse_positive_length, Property::MinChildLeft),
            Property::MinChildRight(_) => parse_property(&mut parser, parse_positive_length, Property::MinChildRight),
            Property::MinChildTop(_) => parse_property(&mut parser, parse_positive_length, Property::MinChildTop),
            Property::MinHeight(_) => parse_property(&mut parser, parse_positive_length, Property::MinHeight),
            Property::MinLeft(_) => parse_property(&mut parser, parse_positive_length, Property::MinLeft),
            Property::MinRight(_) => parse_property(&mut parser, parse_positive_length, Property::MinRight),
            Property::MinTop(_) => parse_property(&mut parser, parse_positive_length, Property::MinTop),
            Property::MinWidth(_) => parse_property(&mut parser, parse_positive_length, Property::MinWidth),
            Property::Opacity(_) => parse_property(&mut parser, parse_opacity, Property::Opacity),
            Property::Outline(_) => parse_shorthand(&mut parser, parse_outline_sh, Property::Outline),
            Property::OutlineColor(_) => parse_property(&mut parser, parse_color, Property::OutlineColor),
            Property::OutlineOffset(_) => parse_property(&mut parser, parse_length, Property::OutlineOffset),
            Property::OutlineWidth(_) => parse_property(&mut parser, parse_positive_length, Property::OutlineWidth),
            Property::Position(_) => parse_property(&mut parser, parse_position, Property::Position),
            Property::Right(_) => parse_property(&mut parser, parse_unit, Property::Right),
            Property::SelectionBackground(_) => parse_property(&mut parser, parse_color, Property::SelectionBackground),
            Property::SelectionColor(_) => parse_property(&mut parser, parse_color, Property::SelectionColor),
            Property::Space(_) => parse_shorthand(&mut parser, parse_space_sh, Property::Space),
            Property::TextAlign(_) => parse_property(&mut parser, parse_text_align, Property::TextAlign),
            Property::TextShadow(_) => parse_property(&mut parser, parse_text_shadow, Property::TextShadow),
            Property::Top(_) => parse_property(&mut parser, parse_unit, Property::Top),
            Property::Transform(_) => parse_property(&mut parser, parse_transform, Property::Transform),
            Property::Width(_) => parse_property(&mut parser, parse_positive_unit, Property::Width),
            Property::WordSpacing(_) => parse_property(&mut parser, parse_unit, Property::WordSpacing),
            Property::ZIndex(_) => parse_property(&mut parser, parse_i32, Property::ZIndex),
        }
    }

    fn apply_resolved(&self, style: &mut Style, parent_style: Option<&Style>) {
        let def = Style::default();
        let current_color = style.color;

        match self {
            Property::BackgroundColor(v) => apply_color(v, &mut style.background_color, parent_style, &def, current_color, |s| &s.background_color),
            Property::BackgroundImage(v) => apply_opt(v, &mut style.background_image, parent_style, &def, |s| &s.background_image),
            Property::Border(_) => {}
            Property::BorderBottom(_) => {}
            Property::BorderBottomColor(v) => apply_color(v, &mut style.border_bottom_color, parent_style, &def, current_color, |s| &s.border_bottom_color),
            Property::BorderBottomLeftRadius(v) => apply_value(v, &mut style.border_bottom_left_radius, parent_style, &def, |s| &s.border_bottom_left_radius),
            Property::BorderBottomRightRadius(v) => {
                apply_value(v, &mut style.border_bottom_right_radius, parent_style, &def, |s| &s.border_bottom_right_radius)
            }
            Property::BorderBottomWidth(v) => apply_value(v, &mut style.border_bottom_width, parent_style, &def, |s| &s.border_bottom_width),
            Property::BorderColor(_) => {}
            Property::BorderLeft(_) => {}
            Property::BorderLeftColor(v) => apply_color(v, &mut style.border_left_color, parent_style, &def, current_color, |s| &s.border_left_color),
            Property::BorderLeftWidth(v) => apply_value(v, &mut style.border_left_width, parent_style, &def, |s| &s.border_left_width),
            Property::BorderRadius(_) => {}
            Property::BorderRight(_) => {}
            Property::BorderRightColor(v) => apply_color(v, &mut style.border_right_color, parent_style, &def, current_color, |s| &s.border_right_color),
            Property::BorderRightWidth(v) => apply_value(v, &mut style.border_right_width, parent_style, &def, |s| &s.border_right_width),
            Property::BorderTop(_) => {}
            Property::BorderTopColor(v) => apply_color(v, &mut style.border_top_color, parent_style, &def, current_color, |s| &s.border_top_color),
            Property::BorderTopLeftRadius(v) => apply_value(v, &mut style.border_top_left_radius, parent_style, &def, |s| &s.border_top_left_radius),
            Property::BorderTopRightRadius(v) => apply_value(v, &mut style.border_top_right_radius, parent_style, &def, |s| &s.border_top_right_radius),
            Property::BorderTopWidth(v) => apply_value(v, &mut style.border_top_width, parent_style, &def, |s| &s.border_top_width),
            Property::BorderWidth(_) => {}
            Property::Bottom(v) => apply_value(v, &mut style.bottom, parent_style, &def, |s| &s.bottom),
            Property::BoxShadow(v) => apply_opt(v, &mut style.box_shadow, parent_style, &def, |s| &s.box_shadow),
            Property::ChildBetween(v) => apply_value(v, &mut style.child_between, parent_style, &def, |s| &s.child_between),
            Property::ChildBottom(v) => apply_value(v, &mut style.child_bottom, parent_style, &def, |s| &s.child_bottom),
            Property::ChildLeft(v) => apply_value(v, &mut style.child_left, parent_style, &def, |s| &s.child_left),
            Property::ChildRight(v) => apply_value(v, &mut style.child_right, parent_style, &def, |s| &s.child_right),
            Property::ChildSpace(_) => {}
            Property::ChildTop(v) => apply_value(v, &mut style.child_top, parent_style, &def, |s| &s.child_top),
            Property::Color(v) => apply_color(v, &mut style.color, parent_style, &def, current_color, |s| &s.color),
            Property::Display(v) => apply_value(v, &mut style.display, parent_style, &def, |s| &s.display),
            Property::FlexBasis(v) => apply_value(v, &mut style.flex_basis, parent_style, &def, |s| &s.flex_basis),
            Property::Font(_) => {}
            Property::FontFamily(v) => apply_opt(v, &mut style.font_family, parent_style, &def, |s| &s.font_family),
            Property::FontSize(v) => apply_value(v, &mut style.font_size, parent_style, &def, |s| &s.font_size),
            Property::FontStyle(v) => apply_value(v, &mut style.font_style, parent_style, &def, |s| &s.font_style),
            Property::FontWeight(v) => apply_value(v, &mut style.font_weight, parent_style, &def, |s| &s.font_weight),
            Property::FontWidth(v) => apply_value(v, &mut style.font_width, parent_style, &def, |s| &s.font_width),
            Property::Height(v) => apply_value(v, &mut style.height, parent_style, &def, |s| &s.height),
            Property::Left(v) => apply_value(v, &mut style.left, parent_style, &def, |s| &s.left),
            Property::LetterSpacing(v) => apply_opt_unit(v, &mut style.letter_spacing, parent_style, &def, |s| &s.letter_spacing),
            Property::LineHeight(v) => apply_value(v, &mut style.line_height, parent_style, &def, |s| &s.line_height),
            Property::MaxBottom(v) => apply_opt(v, &mut style.max_bottom, parent_style, &def, |s| &s.max_bottom),
            Property::MaxChildBetween(v) => apply_opt(v, &mut style.max_child_between, parent_style, &def, |s| &s.max_child_between),
            Property::MaxChildBottom(v) => apply_opt(v, &mut style.max_child_bottom, parent_style, &def, |s| &s.max_child_bottom),
            Property::MaxChildLeft(v) => apply_opt(v, &mut style.max_child_left, parent_style, &def, |s| &s.max_child_left),
            Property::MaxChildRight(v) => apply_opt(v, &mut style.max_child_right, parent_style, &def, |s| &s.max_child_right),
            Property::MaxChildTop(v) => apply_opt(v, &mut style.max_child_top, parent_style, &def, |s| &s.max_child_top),
            Property::MaxHeight(v) => apply_opt(v, &mut style.max_height, parent_style, &def, |s| &s.max_height),
            Property::MaxLeft(v) => apply_opt(v, &mut style.max_left, parent_style, &def, |s| &s.max_left),
            Property::MaxRight(v) => apply_opt(v, &mut style.max_right, parent_style, &def, |s| &s.max_right),
            Property::MaxTop(v) => apply_opt(v, &mut style.max_top, parent_style, &def, |s| &s.max_top),
            Property::MaxWidth(v) => apply_opt(v, &mut style.max_width, parent_style, &def, |s| &s.max_width),
            Property::MinBottom(v) => apply_opt(v, &mut style.min_bottom, parent_style, &def, |s| &s.min_bottom),
            Property::MinChildBetween(v) => apply_opt(v, &mut style.min_child_between, parent_style, &def, |s| &s.min_child_between),
            Property::MinChildBottom(v) => apply_opt(v, &mut style.min_child_bottom, parent_style, &def, |s| &s.min_child_bottom),
            Property::MinChildLeft(v) => apply_opt(v, &mut style.min_child_left, parent_style, &def, |s| &s.min_child_left),
            Property::MinChildRight(v) => apply_opt(v, &mut style.min_child_right, parent_style, &def, |s| &s.min_child_right),
            Property::MinChildTop(v) => apply_opt(v, &mut style.min_child_top, parent_style, &def, |s| &s.min_child_top),
            Property::MinHeight(v) => apply_opt(v, &mut style.min_height, parent_style, &def, |s| &s.min_height),
            Property::MinLeft(v) => apply_opt(v, &mut style.min_left, parent_style, &def, |s| &s.min_left),
            Property::MinRight(v) => apply_opt(v, &mut style.min_right, parent_style, &def, |s| &s.min_right),
            Property::MinTop(v) => apply_opt(v, &mut style.min_top, parent_style, &def, |s| &s.min_top),
            Property::MinWidth(v) => apply_opt(v, &mut style.min_width, parent_style, &def, |s| &s.min_width),
            Property::Opacity(v) => apply_value(v, &mut style.opacity, parent_style, &def, |s| &s.opacity),
            Property::Outline(_) => {}
            Property::OutlineColor(v) => apply_color(v, &mut style.outline_color, parent_style, &def, current_color, |s| &s.outline_color),
            Property::OutlineOffset(v) => apply_value(v, &mut style.outline_offset, parent_style, &def, |s| &s.outline_offset),
            Property::OutlineWidth(v) => apply_value(v, &mut style.outline_width, parent_style, &def, |s| &s.outline_width),
            Property::Position(v) => apply_value(v, &mut style.position, parent_style, &def, |s| &s.position),
            Property::Right(v) => apply_value(v, &mut style.right, parent_style, &def, |s| &s.right),
            Property::SelectionBackground(v) => apply_color(v, &mut style.selection_background, parent_style, &def, current_color, |s| &s.selection_background),
            Property::SelectionColor(v) => apply_opt_color(v, &mut style.selection_color, parent_style, &def, current_color, |s| &s.selection_color),
            Property::Space(_) => {}
            Property::TextAlign(v) => apply_value(v, &mut style.text_align, parent_style, &def, |s| &s.text_align),
            Property::TextShadow(v) => apply_opt(v, &mut style.text_shadow, parent_style, &def, |s| &s.text_shadow),
            Property::Top(v) => apply_value(v, &mut style.top, parent_style, &def, |s| &s.top),
            Property::Transform(v) => apply_value(v, &mut style.transform, parent_style, &def, |s| &s.transform),
            Property::Width(v) => apply_value(v, &mut style.width, parent_style, &def, |s| &s.width),
            Property::WordSpacing(v) => apply_opt_unit(v, &mut style.word_spacing, parent_style, &def, |s| &s.word_spacing),
            Property::ZIndex(v) => apply_value(v, &mut style.z_index, parent_style, &def, |s| &s.z_index),
        }
    }
}

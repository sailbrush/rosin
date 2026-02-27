use std::{path::Path, sync::Arc};

use cssparser::{color::PredefinedColorSpace, *};
use cssparser_color::{DefaultColorParser, parse_color_with};
use kurbo::Affine;
use smallvec::{SmallVec, smallvec};

use crate::{
    css::{self, properties::*, *},
    interner::StringInterner,
    peniko::{
        self,
        color::{
            A98Rgb, AlphaColor, ColorSpace, ColorSpaceTag, DisplayP3, DynamicColor, Hsl, HueDirection, Hwb, Lab, Lch, LinearSrgb, Oklab, Oklch, ProphotoRgb,
            Rec2020, Srgb, XyzD50, XyzD65,
        },
    },
};

pub(crate) type Props = SmallVec<[Property; 2]>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CustomParseError {
    VarFunction,
    InvalidValue,
    UnsupportedValue,
}

impl From<()> for CustomParseError {
    fn from(_: ()) -> Self {
        CustomParseError::InvalidValue
    }
}

// ---------- Rules Parser ----------

pub(crate) struct RulesParser<'i> {
    pub(crate) file_name: Option<&'i Path>,
}

impl<'i> RuleBodyItemParser<'i, Vec<Rule>, CustomParseError> for RulesParser<'i> {
    fn parse_declarations(&self) -> bool {
        false
    }

    fn parse_qualified(&self) -> bool {
        true
    }
}

impl<'i> AtRuleParser<'i> for RulesParser<'i> {
    type Prelude = ();
    type AtRule = Vec<Rule>;
    type Error = CustomParseError;
}

impl<'i> DeclarationParser<'i> for RulesParser<'i> {
    type Declaration = Vec<Rule>;
    type Error = CustomParseError;
}

impl<'i> QualifiedRuleParser<'i> for RulesParser<'i> {
    type Prelude = Vec<Vec<Selector>>;
    type QualifiedRule = Vec<Rule>;
    type Error = CustomParseError;

    fn parse_prelude<'t>(&mut self, parser: &mut Parser<'i, '_>) -> Result<Self::Prelude, ParseError<'i, Self::Error>> {
        let mut interner = StringInterner::global().write();
        parser.parse_comma_separated(|parser| {
            let mut selectors: Vec<Selector> = Vec::new();
            let mut pending: Option<Selector> = None;
            let mut found_class = false;

            while !parser.is_exhausted() {
                match parser.next_including_whitespace()? {
                    Token::WhiteSpace(_) => {
                        if found_class && pending.is_none() {
                            pending = Some(Selector::Descendant);
                        }
                    }
                    Token::Delim('>') => {
                        pending = Some(Selector::Child);
                    }
                    Token::Delim('*') => {
                        if let Some(c) = pending.take() {
                            selectors.push(c);
                        }
                        selectors.push(Selector::Wildcard);
                        found_class = true;
                    }
                    Token::Colon => {
                        let s = parser.expect_ident()?;
                        if let Some(c) = pending.take() {
                            selectors.push(c);
                        }
                        match_ignore_ascii_case! { s,
                            "focus" => selectors.push(Selector::Focus),
                            "hover" => selectors.push(Selector::Hover),
                            "active" => selectors.push(Selector::Active),
                            "disabled" => selectors.push(Selector::Disabled),
                            "enabled" => selectors.push(Selector::Enabled),
                            _ => return Err(parser.new_error_for_next_token()),
                        }
                        found_class = true;
                    }
                    Token::Ident(s) => {
                        if let Some(c) = pending.take() {
                            selectors.push(c);
                        }
                        selectors.push(Selector::Class(interner.intern(s.as_ref())));
                        found_class = true;
                    }
                    Token::Delim('.') => {} // intentionally treat element selectors as class selectors
                    _ => return Err(parser.new_error_for_next_token()),
                }
            }
            if pending == Some(Selector::Descendant) {
                pending = None;
            }
            if !found_class || pending.is_some() {
                return Err(parser.new_error(BasicParseErrorKind::EndOfInput));
            }
            Ok(selectors)
        })
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
        parser: &mut Parser<'i, '_>,
    ) -> Result<Self::QualifiedRule, ParseError<'i, Self::Error>> {
        let mut property_list = Props::with_capacity(32);
        let mut property_parser = PropertiesParser { variables: Vec::new() };
        for result in RuleBodyParser::new(parser, &mut property_parser) {
            match result {
                Ok(property) => property_list.extend(property),
                Err((error, css)) => {
                    let base_msg = match error.kind {
                        cssparser::ParseErrorKind::Custom(CustomParseError::UnsupportedValue) => "Unsupported CSS value",
                        _ => "Failed to parse CSS property",
                    };
                    let snippet = css.lines().next().unwrap_or("");
                    let msg = format_args!("{base_msg}: `{snippet}`");
                    css::log_error(msg, error.location, self.file_name);
                }
            }
        }
        let property_list = Arc::new(property_list);

        let mut rules = Vec::new();
        for selectors in prelude {
            let mut specificity = 0;
            let mut has_pseudos = false;

            for selector in &selectors {
                match selector {
                    Selector::Class(_) => {
                        specificity += 10;
                    }
                    Selector::Hover | Selector::Focus | Selector::Active | Selector::Enabled | Selector::Disabled => {
                        specificity += 10;
                        has_pseudos = true;
                    }
                    _ => {}
                }
            }

            rules.push(Rule {
                specificity,
                selectors,
                properties: property_list.clone(),
                has_pseudos,
                variables: property_parser.variables.to_vec(),
            });
        }

        Ok(rules)
    }
}

// ---------- Properties Parser ----------

pub(crate) struct PropertiesParser {
    pub(crate) variables: Vec<(Arc<str>, Arc<str>)>,
}

impl<'i> RuleBodyItemParser<'i, Props, CustomParseError> for PropertiesParser {
    fn parse_declarations(&self) -> bool {
        true
    }

    fn parse_qualified(&self) -> bool {
        false
    }
}

impl<'i> AtRuleParser<'i> for PropertiesParser {
    type Prelude = ();
    type AtRule = Props;
    type Error = CustomParseError;
}

impl<'i> QualifiedRuleParser<'i> for PropertiesParser {
    type Prelude = ();
    type QualifiedRule = Props;
    type Error = CustomParseError;
}

impl<'i> DeclarationParser<'i> for PropertiesParser {
    type Declaration = Props;
    type Error = CustomParseError;

    fn parse_value<'t>(&mut self, name: CowRcStr<'i>, parser: &mut Parser<'i, '_>, _: &ParserState) -> Result<Self::Declaration, ParseError<'i, Self::Error>> {
        // Capture CSS variables
        if name.starts_with("--") {
            parser.skip_whitespace();
            let start = parser.position();
            consume_value(parser)?;
            let value = parser.slice_from(start).trim().to_string();
            self.variables.push((Arc::from(name.as_ref()), Arc::from(value.as_ref())));
            return Ok(Props::new());
        }

        match_ignore_ascii_case! { &name,
            "background-color" => parse_property(parser, parse_color, Property::BackgroundColor),
            "background-image" => parse_property(parser, parse_background_image, Property::BackgroundImage),
            "border-bottom-color" => parse_property(parser, parse_color, Property::BorderBottomColor),
            "border-bottom-left-radius" => parse_property(parser, parse_positive_length, Property::BorderBottomLeftRadius),
            "border-bottom-right-radius" => parse_property(parser, parse_positive_length, Property::BorderBottomRightRadius),
            "border-bottom-width" => parse_property(parser, parse_positive_length, Property::BorderBottomWidth),
            "border-bottom" => parse_shorthand(parser, |p| parse_border_side_sh(p, BorderSide::Bottom), Property::BorderBottom),
            "border-color" => parse_shorthand(parser, parse_border_color_sh, Property::BorderColor),
            "border-left-color" => parse_property(parser, parse_color, Property::BorderLeftColor),
            "border-left-width" => parse_property(parser, parse_positive_length, Property::BorderLeftWidth),
            "border-left" => parse_shorthand(parser, |p| parse_border_side_sh(p, BorderSide::Left), Property::BorderLeft),
            "border-radius" => parse_shorthand(parser, parse_border_radius_sh, Property::BorderRadius),
            "border-right-color" => parse_property(parser, parse_color, Property::BorderRightColor),
            "border-right-width" => parse_property(parser, parse_positive_length, Property::BorderRightWidth),
            "border-right" => parse_shorthand(parser, |p| parse_border_side_sh(p, BorderSide::Right), Property::BorderRight),
            "border-top-color" => parse_property(parser, parse_color, Property::BorderTopColor),
            "border-top-left-radius" => parse_property(parser, parse_positive_length, Property::BorderTopLeftRadius),
            "border-top-right-radius" => parse_property(parser, parse_positive_length, Property::BorderTopRightRadius),
            "border-top-width" => parse_property(parser, parse_positive_length, Property::BorderTopWidth),
            "border-top" => parse_shorthand(parser, |p| parse_border_side_sh(p, BorderSide::Top), Property::BorderTop),
            "border-width" => parse_shorthand(parser, parse_border_width_sh, Property::BorderWidth),
            "border" => parse_shorthand(parser, parse_border_sh, Property::Border),
            "bottom" => parse_property(parser, parse_unit, Property::Bottom),
            "left" => parse_property(parser, parse_unit, Property::Left),
            "right" => parse_property(parser, parse_unit, Property::Right),
            "top" => parse_property(parser, parse_unit, Property::Top),
            "box-shadow" => parse_property(parser, parse_box_shadow, Property::BoxShadow),
            "child-between" => parse_property(parser, parse_positive_unit, Property::ChildBetween),
            "child-bottom" => parse_property(parser, parse_positive_unit, Property::ChildBottom),
            "child-left" => parse_property(parser, parse_positive_unit, Property::ChildLeft),
            "child-right" => parse_property(parser, parse_positive_unit, Property::ChildRight),
            "child-space" => parse_shorthand(parser, parse_child_space_sh, Property::ChildSpace),
            "child-top" => parse_property(parser, parse_positive_unit, Property::ChildTop),
            "color" => parse_property(parser, parse_color, Property::Color),
            "display" => parse_property(parser, parse_display, Property::Display),
            "flex-basis" => parse_property(parser, parse_positive_length, Property::FlexBasis),
            "font-family" => parse_property(parser, parse_font_family, Property::FontFamily),
            "font-size" => parse_property(parser, parse_font_size, Property::FontSize),
            "font-width" => parse_property(parser, parse_font_width, Property::FontWidth),
            "font-style" => parse_property(parser, parse_font_style, Property::FontStyle),
            "font-weight" => parse_property(parser, parse_font_weight, Property::FontWeight),
            "font" => parse_shorthand(parser, parse_font_sh, Property::Font),
            "height" => parse_property(parser, parse_positive_unit, Property::Height),
            "width" => parse_property(parser, parse_positive_unit, Property::Width),
            "letter-spacing" => parse_property(parser, parse_unit, Property::LetterSpacing),
            "word-spacing" => parse_property(parser, parse_unit, Property::WordSpacing),
            "line-height" => parse_property(parser, parse_positive_unit, Property::LineHeight),
            "max-bottom" => parse_property(parser, parse_positive_length, Property::MaxBottom),
            "max-child-between" => parse_property(parser, parse_positive_length, Property::MaxChildBetween),
            "max-child-bottom" => parse_property(parser, parse_positive_length, Property::MaxChildBottom),
            "max-child-left" => parse_property(parser, parse_positive_length, Property::MaxChildLeft),
            "max-child-right" => parse_property(parser, parse_positive_length, Property::MaxChildRight),
            "max-child-top" => parse_property(parser, parse_positive_length, Property::MaxChildTop),
            "max-height" => parse_property(parser, parse_positive_length, Property::MaxHeight),
            "max-left" => parse_property(parser, parse_positive_length, Property::MaxLeft),
            "max-right" => parse_property(parser, parse_positive_length, Property::MaxRight),
            "max-top" => parse_property(parser, parse_positive_length, Property::MaxTop),
            "max-width" => parse_property(parser, parse_positive_length, Property::MaxWidth),
            "min-bottom" => parse_property(parser, parse_positive_length, Property::MinBottom),
            "min-child-between" => parse_property(parser, parse_positive_length, Property::MinChildBetween),
            "min-child-bottom" => parse_property(parser, parse_positive_length, Property::MinChildBottom),
            "min-child-left" => parse_property(parser, parse_positive_length, Property::MinChildLeft),
            "min-child-right" => parse_property(parser, parse_positive_length, Property::MinChildRight),
            "min-child-top" => parse_property(parser, parse_positive_length, Property::MinChildTop),
            "min-height" => parse_property(parser, parse_positive_length, Property::MinHeight),
            "min-left" => parse_property(parser, parse_positive_length, Property::MinLeft),
            "min-right" => parse_property(parser, parse_positive_length, Property::MinRight),
            "min-top" => parse_property(parser, parse_positive_length, Property::MinTop),
            "min-width" => parse_property(parser, parse_positive_length, Property::MinWidth),
            "opacity" => parse_property(parser, parse_opacity, Property::Opacity),
            "outline-color" => parse_property(parser, parse_color, Property::OutlineColor),
            "outline-offset" => parse_property(parser, parse_length, Property::OutlineOffset),
            "outline-width" => parse_property(parser, parse_positive_length, Property::OutlineWidth),
            "outline" => parse_shorthand(parser, parse_outline_sh, Property::Outline),
            "position" => parse_property(parser, parse_position, Property::Position),
            "selection-background" => parse_property(parser, parse_color, Property::SelectionBackground),
            "selection-color" => parse_property(parser, parse_color, Property::SelectionColor),
            "space" => parse_shorthand(parser, parse_space_sh, Property::Space),
            "text-align" => parse_property(parser, parse_text_align, Property::TextAlign),
            "text-shadow" => parse_property(parser, parse_text_shadow, Property::TextShadow),
            "transform" => parse_property(parser, parse_transform, Property::Transform),
            "z-index" => parse_property(parser, parse_i32, Property::ZIndex),
            _ => Err(parser.new_error_for_next_token()),
        }
    }
}

// ---------- Top Level Parsers ----------

/// Responsible for checking if a property is set to initial or inherit,
/// deferring parse if needed, and ensuring parser is exhausted.
pub(crate) fn parse_property<'i, T>(
    parser: &mut Parser<'i, '_>,
    parse_exact_value: fn(&mut Parser<'i, '_>) -> Result<PropertyValue<T>, cssparser::ParseError<'i, CustomParseError>>,
    variant: fn(PropertyValue<T>) -> Property,
) -> Result<Props, cssparser::ParseError<'i, CustomParseError>> {
    if is_keyword_exhausted(parser, "initial") {
        return Ok(smallvec![variant(PropertyValue::Initial)]);
    }
    if is_keyword_exhausted(parser, "inherit") {
        return Ok(smallvec![variant(PropertyValue::Inherit)]);
    }
    let value = exact_or_deferred(parser, parse_exact_value, PropertyValue::Deferred)?;
    Ok(smallvec![variant(value)])
}

/// Responsible for deferring parse if needed, and ensuring parser is exhausted.
/// Shorthand parsers must check for initial or inherit themselves because of mixed return types.
pub(crate) fn parse_shorthand<'i>(
    parser: &mut Parser<'i, '_>,
    parse_exact: fn(&mut Parser<'i, '_>) -> Result<Props, cssparser::ParseError<'i, CustomParseError>>,
    variant: fn(PropertyValue<NoExact>) -> Property,
) -> Result<Props, cssparser::ParseError<'i, CustomParseError>> {
    exact_or_deferred(parser, parse_exact, |raw, loc| smallvec![variant(PropertyValue::Deferred(raw, loc))])
}

fn exact_or_deferred<'i, R>(
    parser: &mut Parser<'i, '_>,
    parse_exact: impl FnOnce(&mut Parser<'i, '_>) -> Result<R, cssparser::ParseError<'i, CustomParseError>>,
    deferred: impl FnOnce(Arc<str>, SourceLocation) -> R,
) -> Result<R, cssparser::ParseError<'i, CustomParseError>> {
    let start_state = parser.state();

    let parsed = parser.try_parse(|p| -> Result<R, cssparser::ParseError<'i, CustomParseError>> {
        let v = parse_exact(p)?;
        p.expect_exhausted()?;
        Ok(v)
    });

    match parsed {
        Ok(v) => Ok(v),
        Err(e) => {
            if matches!(e.kind, cssparser::ParseErrorKind::Custom(CustomParseError::VarFunction)) {
                parser.reset(&start_state);
                parser.skip_whitespace();
                let loc = parser.current_source_location();
                let start_pos = parser.position();
                consume_value(parser)?;
                let raw = Arc::from(parser.slice_from(start_pos));
                return Ok(deferred(raw, loc));
            }
            Err(e)
        }
    }
}

fn consume_value<'i>(parser: &mut Parser<'i, '_>) -> Result<(), cssparser::ParseError<'i, CustomParseError>> {
    while let Ok(token) = parser.next_including_whitespace() {
        match token {
            Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                parser.parse_nested_block(|p| consume_value(p))?;
            }
            _ => {}
        }
    }
    Ok(())
}

// ---------- Helpers ----------

#[derive(Copy, Clone)]
pub(crate) enum BorderSide {
    Top,
    Right,
    Bottom,
    Left,
}

const BORDER_COLORS: &[fn(PropertyValue<ColorProperty>) -> Property] = &[
    Property::BorderTopColor,
    Property::BorderRightColor,
    Property::BorderBottomColor,
    Property::BorderLeftColor,
];

const BORDER_WIDTHS: &[fn(PropertyValue<Length>) -> Property] = &[
    Property::BorderTopWidth,
    Property::BorderRightWidth,
    Property::BorderBottomWidth,
    Property::BorderLeftWidth,
];

const BORDER_RADII: &[fn(PropertyValue<Length>) -> Property] = &[
    Property::BorderTopLeftRadius,
    Property::BorderTopRightRadius,
    Property::BorderBottomRightRadius,
    Property::BorderBottomLeftRadius,
];

const SPACE_PROPS: &[fn(PropertyValue<Unit>) -> Property] = &[Property::Top, Property::Right, Property::Bottom, Property::Left];
const CHILD_SPACE_PROPS: &[fn(PropertyValue<Unit>) -> Property] = &[Property::ChildTop, Property::ChildRight, Property::ChildBottom, Property::ChildLeft];

fn push_property_array<T: Clone>(props: &[fn(PropertyValue<T>) -> Property], v: PropertyValue<T>, out: &mut Props) {
    for prop in props {
        out.push(prop(v.clone()));
    }
}

#[inline]
fn optional_comma<'i>(p: &mut Parser<'i, '_>) {
    let _ = p.try_parse(|p2| p2.expect_comma());
}

fn is_keyword_exhausted<'i>(parser: &mut Parser<'i, '_>, kw: &'static str) -> bool {
    parser
        .try_parse(|p| {
            p.expect_ident_matching(kw)?;
            p.expect_exhausted()
        })
        .is_ok()
}

fn next_number<'i>(p: &mut Parser<'i, '_>) -> Result<f64, ParseError<'i, CustomParseError>> {
    match p.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(p.new_custom_error(CustomParseError::VarFunction)),
        Token::Number { value, .. } => Ok(*value as f64),
        _ => Err(p.new_error_for_next_token()),
    }
}

fn next_length<'i>(parser: &mut Parser<'i, '_>) -> Result<Length, cssparser::ParseError<'i, CustomParseError>> {
    match parser.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(parser.new_custom_error(CustomParseError::VarFunction)),
        Token::Dimension { value, unit, .. } => Ok(match_ignore_ascii_case! { unit,
            "px" => Length::Px(*value),
            "em" => Length::Em(*value),
            _ => return Err(parser.new_error_for_next_token()),
        }),
        Token::Number { value, .. } if *value == 0.0 => Ok(Length::Px(0.0)),
        _ => Err(parser.new_error_for_next_token()),
    }
}

fn next_positive_length<'i>(p: &mut Parser<'i, '_>) -> Result<Length, ParseError<'i, CustomParseError>> {
    let len = next_length(p)?;
    let is_neg = match len {
        Length::Px(x) => x < 0.0,
        Length::Em(x) => x < 0.0,
    };
    if is_neg {
        return Err(p.new_custom_error(CustomParseError::InvalidValue));
    }
    Ok(len)
}

fn next_angle_radians<'i>(p: &mut Parser<'i, '_>) -> Result<f64, ParseError<'i, CustomParseError>> {
    match p.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(p.new_custom_error(CustomParseError::VarFunction)),
        Token::Number { value, .. } if *value == 0.0 => Ok(0.0),
        Token::Dimension { value, unit, .. } => {
            let v = *value as f64;
            let rad = match_ignore_ascii_case! { unit,
                "rad"  => v,
                "deg"  => v.to_radians(),
                "grad" => (v * 0.9).to_radians(),
                "turn" => v * std::f64::consts::TAU,
                _ => return Err(p.new_custom_error(CustomParseError::UnsupportedValue)),
            };
            Ok(rad)
        }
        _ => Err(p.new_error_for_next_token()),
    }
}

fn next_color<'i>(parser: &mut Parser<'i, '_>) -> Result<ColorProperty, cssparser::ParseError<'i, CustomParseError>> {
    #[inline]
    fn convert<S: ColorSpace>(c1: Option<f32>, c2: Option<f32>, c3: Option<f32>, alpha: Option<f32>) -> ColorProperty {
        let c1 = c1.unwrap_or_default();
        let c2 = c2.unwrap_or_default();
        let c3 = c3.unwrap_or_default();
        let alpha = alpha.unwrap_or(1.0);

        ColorProperty::Color(AlphaColor::<S>::new([c1, c2, c3, alpha]).convert())
    }

    let parsed = parse_color_with(&DefaultColorParser, parser).map_err(|e| match &e.kind {
        ParseErrorKind::Basic(BasicParseErrorKind::UnexpectedToken(Token::Ident(ident) | Token::Function(ident)))
            if ident.as_ref().eq_ignore_ascii_case("var") =>
        {
            e.location.new_custom_error(CustomParseError::VarFunction)
        }

        _ => e.into::<CustomParseError>(),
    })?;

    Ok(match parsed {
        cssparser_color::Color::CurrentColor => ColorProperty::CurrentColor,
        cssparser_color::Color::Rgba(rgba) => {
            ColorProperty::Color(AlphaColor::<Srgb>::from_rgba8(rgba.red, rgba.green, rgba.blue, (rgba.alpha.clamp(0.0, 1.0) * 255.0).round() as u8))
        }
        cssparser_color::Color::Hsl(hsl) => convert::<Hsl>(hsl.hue, hsl.saturation.map(|v| v * 100.0), hsl.lightness.map(|v| v * 100.0), hsl.alpha),
        cssparser_color::Color::Hwb(hwb) => convert::<Hwb>(hwb.hue, hwb.whiteness.map(|v| v * 100.0), hwb.blackness.map(|v| v * 100.0), hwb.alpha),
        cssparser_color::Color::Lab(lab) => convert::<Lab>(lab.lightness, lab.a, lab.b, lab.alpha),
        cssparser_color::Color::Lch(lch) => convert::<Lch>(lch.lightness, lch.chroma, lch.hue, lch.alpha),
        cssparser_color::Color::Oklab(oklab) => convert::<Oklab>(oklab.lightness, oklab.a, oklab.b, oklab.alpha),
        cssparser_color::Color::Oklch(oklch) => convert::<Oklch>(oklch.lightness, oklch.chroma, oklch.hue, oklch.alpha),
        cssparser_color::Color::ColorFunction(func) => match func.color_space {
            PredefinedColorSpace::Srgb => convert::<Srgb>(func.c1, func.c2, func.c3, func.alpha),
            PredefinedColorSpace::SrgbLinear => convert::<LinearSrgb>(func.c1, func.c2, func.c3, func.alpha),
            PredefinedColorSpace::DisplayP3 => convert::<DisplayP3>(func.c1, func.c2, func.c3, func.alpha),
            PredefinedColorSpace::A98Rgb => convert::<A98Rgb>(func.c1, func.c2, func.c3, func.alpha),
            PredefinedColorSpace::ProphotoRgb => convert::<ProphotoRgb>(func.c1, func.c2, func.c3, func.alpha),
            PredefinedColorSpace::Rec2020 => convert::<Rec2020>(func.c1, func.c2, func.c3, func.alpha),
            PredefinedColorSpace::XyzD50 => convert::<XyzD50>(func.c1, func.c2, func.c3, func.alpha),
            PredefinedColorSpace::XyzD65 => convert::<XyzD65>(func.c1, func.c2, func.c3, func.alpha),
        },
    })
}

fn quad_values<'i, T>(
    parser: &mut Parser<'i, '_>,
    mut parse_one: impl FnMut(&mut Parser<'i, '_>) -> Result<PropertyValue<T>, ParseError<'i, CustomParseError>>,
) -> Result<[PropertyValue<T>; 4], ParseError<'i, CustomParseError>>
where
    T: Clone,
{
    let v1 = parse_one(parser)?;
    if parser.is_exhausted() {
        return Ok([v1.clone(), v1.clone(), v1.clone(), v1]);
    }

    let v2 = parse_one(parser)?;
    if parser.is_exhausted() {
        return Ok([v1.clone(), v2.clone(), v1, v2]);
    }

    let v3 = parse_one(parser)?;
    if parser.is_exhausted() {
        return Ok([v1, v2.clone(), v3, v2]);
    }

    let v4 = parse_one(parser)?;
    if !parser.is_exhausted() {
        return Err(parser.new_error_for_next_token());
    }

    Ok([v1, v2, v3, v4])
}

fn shadow_list<'i, T>(
    parser: &mut Parser<'i, '_>,
    allow_inset: bool,
    mut parse_one: impl FnMut(Option<peniko::Color>, bool, Length, Length, Option<Length>, Option<Length>) -> Result<T, ()>,
) -> Result<PropertyValue<Arc<[T]>>, cssparser::ParseError<'i, CustomParseError>> {
    if is_keyword_exhausted(parser, "none") {
        return Ok(PropertyValue::Initial);
    }

    let mut out: Vec<T> = Vec::new();

    struct State {
        vals: [Length; 4],
        vlen: usize,
        color: Option<peniko::Color>,
        inset: bool,
        seen_color: bool,
        seen_inset: bool,
    }

    impl State {
        fn reset(&mut self) {
            self.vlen = 0;
            self.color = None;
            self.inset = false;
            self.seen_color = false;
            self.seen_inset = false;
        }
    }

    let mut st = State {
        vals: [Length::Px(0.0); 4],
        vlen: 0,
        color: None,
        inset: false,
        seen_color: false,
        seen_inset: false,
    };

    // Finish current component and reset component state.
    fn finish_component<'i, T>(
        parser: &mut Parser<'i, '_>,
        st: &mut State,
        out: &mut Vec<T>,
        parse_one: &mut impl FnMut(Option<peniko::Color>, bool, Length, Length, Option<Length>, Option<Length>) -> Result<T, ()>,
    ) -> Result<(), cssparser::ParseError<'i, CustomParseError>> {
        // Need at least offset-x and offset-y
        let (ox, oy, blur, spread) = match st.vlen {
            2 => (st.vals[0], st.vals[1], None, None),
            3 => (st.vals[0], st.vals[1], Some(st.vals[2]), None),
            4 => (st.vals[0], st.vals[1], Some(st.vals[2]), Some(st.vals[3])),
            _ => return Err(parser.new_error_for_next_token()),
        };

        // Reject negative blur radius
        if let Some(b) = blur {
            let neg = match b {
                Length::Px(x) => x < 0.0,
                Length::Em(x) => x < 0.0,
            };
            if neg {
                return Err(parser.new_custom_error(CustomParseError::InvalidValue));
            }
        }

        let item = (*parse_one)(st.color, st.inset, ox, oy, blur, spread).map_err(|_| parser.new_error_for_next_token())?;
        out.push(item);

        st.reset();
        Ok(())
    }

    while !parser.is_exhausted() {
        // Color is allowed anywhere
        match parser.try_parse(|p| next_color(p)) {
            Ok(parsed_color) => {
                if st.seen_color {
                    return Err(parser.new_error_for_next_token());
                }
                st.seen_color = true;
                match parsed_color {
                    ColorProperty::CurrentColor => st.color = None,
                    ColorProperty::Color(parsed) => st.color = Some(parsed),
                }
                continue;
            }
            Err(e) => {
                if cssparser::ParseErrorKind::Custom(CustomParseError::VarFunction) == e.kind {
                    return Err(e);
                }
            }
        }

        if let Ok(val) = parser.try_parse(|p| next_length(p)) {
            if st.vlen == 4 {
                return Err(parser.new_error_for_next_token());
            }
            st.vals[st.vlen] = val;
            st.vlen += 1;
            continue;
        }

        match parser.next()? {
            Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                return Err(parser.new_custom_error(CustomParseError::VarFunction));
            }
            Token::Ident(s) if allow_inset => match_ignore_ascii_case! { s,
                "inset" => {
                    if st.seen_inset {
                        return Err(parser.new_error_for_next_token());
                    }
                    st.seen_inset = true;
                    st.inset = true;
                },
                _ => return Err(parser.new_error_for_next_token()),
            },
            Token::Comma => {
                finish_component(parser, &mut st, &mut out, &mut parse_one)?;
            }
            _ => return Err(parser.new_error_for_next_token()),
        }
    }

    finish_component(parser, &mut st, &mut out, &mut parse_one)?;

    Ok(PropertyValue::Exact(Arc::from(out)))
}

fn stroke_components<'i>(
    parser: &mut Parser<'i, '_>,
    out: &mut Props,
    color_prop: impl Fn(PropertyValue<ColorProperty>) -> Property,
    width_prop: impl Fn(PropertyValue<Length>) -> Property,
) -> Result<(), cssparser::ParseError<'i, CustomParseError>> {
    let mut seen_color = false;
    let mut seen_width = false;
    let mut seen_style = false;

    while !parser.is_exhausted() {
        // Color
        match parser.try_parse(|p| next_color(p)) {
            Ok(color) => {
                if seen_color {
                    return Err(parser.new_error_for_next_token());
                }
                seen_color = true;
                out.push(color_prop(PropertyValue::Exact(color)));
                continue;
            }
            Err(e) => {
                if e.kind == cssparser::ParseErrorKind::Custom(CustomParseError::VarFunction) {
                    return Err(e);
                }
            }
        }

        // Width
        if let Ok(len) = parser.try_parse(|p| next_positive_length(p)) {
            if seen_width {
                return Err(parser.new_error_for_next_token());
            }
            seen_width = true;
            out.push(width_prop(PropertyValue::Exact(len)));
            continue;
        }

        // Style
        match parser.next()? {
            Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                return Err(parser.new_custom_error(CustomParseError::VarFunction));
            }
            Token::Ident(s) => match_ignore_ascii_case! { s,
                "solid" => {
                    if seen_style {
                        return Err(parser.new_error_for_next_token());
                    }
                    seen_style = true;
                },
                "dotted" | "dashed" | "double" | "groove" | "ridge" | "inset" | "outset" => {
                    return Err(parser.new_custom_error(CustomParseError::UnsupportedValue));
                },
                _ => return Err(parser.new_error_for_next_token()),
            },
            _ => return Err(parser.new_error_for_next_token()),
        }
    }

    Ok(())
}

// ---------- Parsers ----------

pub(crate) fn parse_length<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<Length>, cssparser::ParseError<'i, CustomParseError>> {
    Ok(PropertyValue::Exact(next_length(parser)?))
}

pub(crate) fn parse_positive_length<'i>(p: &mut Parser<'i, '_>) -> Result<PropertyValue<Length>, ParseError<'i, CustomParseError>> {
    Ok(PropertyValue::Exact(next_positive_length(p)?))
}

pub(crate) fn parse_color<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<ColorProperty>, cssparser::ParseError<'i, CustomParseError>> {
    Ok(PropertyValue::Exact(next_color(parser)?))
}

pub(crate) fn parse_opacity<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<f32>, cssparser::ParseError<'i, CustomParseError>> {
    Ok(PropertyValue::Exact(match parser.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(parser.new_custom_error(CustomParseError::VarFunction)),
        Token::Percentage { unit_value, .. } => Ok(unit_value.clamp(0.0, 1.0)),
        Token::Number { value, .. } => Ok(value.clamp(0.0, 1.0)),
        _ => Err(parser.new_error_for_next_token()),
    }?))
}

pub(crate) fn parse_background_image<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<GradientStack>, cssparser::ParseError<'i, CustomParseError>> {
    if is_keyword_exhausted(parser, "none") {
        return Ok(PropertyValue::Initial);
    }

    let gradients = parser.parse_comma_separated(|parser| {
        let Token::Function(name) = parser.next()? else {
            return Err(parser.new_error_for_next_token());
        };

        match_ignore_ascii_case! { name,
            "var" => return Err(parser.new_custom_error(CustomParseError::VarFunction)),
            "linear-gradient" => {},
            _ => return Err(parser.new_error_for_next_token()),
        }

        parser.parse_nested_block(|parser| {
            let mut angle = GradientAngle::ToBottom;
            let mut interpolation_cs = ColorSpaceTag::Srgb;
            let mut hue_direction = HueDirection::default();
            let mut had_prelude = false;

            // Optional <angle> or "to <side-or-corner>"
            match parser.try_parse(|p| {
                if let Ok(a) = p.try_parse(|p2| match p2.next()? {
                    Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(p2.new_custom_error(CustomParseError::VarFunction)),
                    Token::Number { value, .. } if *value == 0.0 => Ok(GradientAngle::Radians(0.0)),
                    Token::Dimension { value, unit, .. } => {
                        let rad = match_ignore_ascii_case! { unit,
                            "deg" => value.to_radians(),
                            "rad" => *value,
                            "grad" => (*value * 0.9).to_radians(),
                            "turn" => *value * std::f32::consts::TAU,
                            _ => return Err(p2.new_error_for_next_token::<CustomParseError>()),
                        };
                        Ok(GradientAngle::Radians(rad))
                    }
                    _ => Err(p2.new_error_for_next_token()),
                }) {
                    return Ok(Some(a));
                }

                match p.next()? {
                    Token::Ident(i) if i.eq_ignore_ascii_case("to") => {}
                    Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                        return Err(p.new_custom_error(CustomParseError::VarFunction));
                    }
                    _ => return Err(p.new_error_for_next_token::<CustomParseError>()),
                }

                const LEFT: u8 = 1 << 0;
                const RIGHT: u8 = 1 << 1;
                const TOP: u8 = 1 << 2;
                const BOTTOM: u8 = 1 << 3;
                const TOP_LEFT: u8 = LEFT | TOP;
                const BOTTOM_LEFT: u8 = LEFT | BOTTOM;
                const TOP_RIGHT: u8 = RIGHT | TOP;
                const BOTTOM_RIGHT: u8 = RIGHT | BOTTOM;

                let mut mask: u8 = 0;

                for _ in 0..2 {
                    let st = p.state();
                    let bit = match p.next() {
                        Ok(Token::Function(name)) if name.eq_ignore_ascii_case("var") => {
                            return Err(p.new_custom_error(CustomParseError::VarFunction));
                        }
                        Ok(Token::Ident(ident)) => match_ignore_ascii_case! { ident,
                            "left" => LEFT,
                            "right" => RIGHT,
                            "top" => TOP,
                            "bottom" => BOTTOM,
                            _ => {
                                p.reset(&st);
                                break;
                            }
                        },
                        _ => {
                            p.reset(&st);
                            break;
                        }
                    };

                    if (mask & bit) != 0 {
                        return Err(p.new_error_for_next_token::<CustomParseError>());
                    }
                    mask |= bit;
                }

                if (mask & (LEFT | RIGHT)) == (LEFT | RIGHT) || (mask & (TOP | BOTTOM)) == (TOP | BOTTOM) {
                    return Err(p.new_error_for_next_token::<CustomParseError>());
                }

                Ok(Some(match mask {
                    LEFT => GradientAngle::ToLeft,
                    RIGHT => GradientAngle::ToRight,
                    TOP => GradientAngle::ToTop,
                    BOTTOM => GradientAngle::ToBottom,
                    TOP_LEFT => GradientAngle::ToTopLeft,
                    BOTTOM_LEFT => GradientAngle::ToBottomLeft,
                    TOP_RIGHT => GradientAngle::ToTopRight,
                    BOTTOM_RIGHT => GradientAngle::ToBottomRight,
                    _ => return Err(p.new_error_for_next_token()),
                }))
            }) {
                Ok(Some(a)) => {
                    angle = a;
                    had_prelude = true;
                }
                Ok(None) => {}
                Err(e) => {
                    if matches!(&e.kind, cssparser::ParseErrorKind::Custom(CustomParseError::VarFunction)) {
                        return Err(e);
                    }
                }
            }

            // Optional "in <color-space> [<hue-interpolation-method>]"
            match parser.try_parse(|p| {
                match p.next()? {
                    Token::Ident(i) if i.eq_ignore_ascii_case("in") => {}
                    Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                        return Err(p.new_custom_error(CustomParseError::VarFunction));
                    }
                    _ => return Err(p.new_error_for_next_token::<CustomParseError>()),
                }

                let cs_ident = match p.next()? {
                    Token::Ident(i) => i,
                    Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                        return Err(p.new_custom_error(CustomParseError::VarFunction));
                    }
                    _ => return Err(p.new_error_for_next_token::<CustomParseError>()),
                };

                let cs: ColorSpaceTag = match_ignore_ascii_case! { cs_ident,
                    "srgb" => ColorSpaceTag::Srgb,
                    "srgb-linear" | "linear-srgb" => ColorSpaceTag::LinearSrgb,
                    "display-p3" => ColorSpaceTag::DisplayP3,
                    "a98-rgb" => ColorSpaceTag::A98Rgb,
                    "prophoto-rgb" => ColorSpaceTag::ProphotoRgb,
                    "rec2020" => ColorSpaceTag::Rec2020,
                    "lab" => ColorSpaceTag::Lab,
                    "lch" => ColorSpaceTag::Lch,
                    "hsl" => ColorSpaceTag::Hsl,
                    "hwb" => ColorSpaceTag::Hwb,
                    "oklab" => ColorSpaceTag::Oklab,
                    "oklch" => ColorSpaceTag::Oklch,
                    "xyz-d50" => ColorSpaceTag::XyzD50,
                    "xyz" | "xyz-d65" => ColorSpaceTag::XyzD65,
                    "acescg" | "aces-cg" => ColorSpaceTag::AcesCg,
                    "aces2065-1" => ColorSpaceTag::Aces2065_1,
                    _ => return Err(p.new_error_for_next_token::<CustomParseError>()),
                };

                let dir = match p.try_parse(|p2| {
                    let which = match p2.next()? {
                        Token::Ident(i) => i,
                        Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                            return Err(p2.new_custom_error(CustomParseError::VarFunction));
                        }
                        _ => return Err(p2.new_error_for_next_token::<CustomParseError>()),
                    };

                    let dir = match_ignore_ascii_case! { which,
                        "shorter" => HueDirection::Shorter,
                        "longer" => HueDirection::Longer,
                        "increasing" => HueDirection::Increasing,
                        "decreasing" => HueDirection::Decreasing,
                        _ => return Err(p2.new_error_for_next_token::<CustomParseError>()),
                    };

                    match p2.next()? {
                        Token::Ident(i) if i.eq_ignore_ascii_case("hue") => {}
                        Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                            return Err(p2.new_custom_error(CustomParseError::VarFunction));
                        }
                        _ => return Err(p2.new_error_for_next_token::<CustomParseError>()),
                    }

                    Ok(dir)
                }) {
                    Ok(d) => d,
                    Err(e) => {
                        if matches!(&e.kind, cssparser::ParseErrorKind::Custom(CustomParseError::VarFunction)) {
                            return Err(e);
                        }
                        HueDirection::default()
                    }
                };

                Ok((cs, dir))
            }) {
                Ok((cs, dir)) => {
                    interpolation_cs = cs;
                    hue_direction = dir;
                    had_prelude = true;
                }
                Err(e) => {
                    // Only propagate var() errors, everything else means no prelude
                    if matches!(&e.kind, cssparser::ParseErrorKind::Custom(CustomParseError::VarFunction)) {
                        return Err(e);
                    }
                }
            }

            if had_prelude {
                parser.expect_comma()?;
            }

            enum StopPiece {
                Color(ColorProperty),
                Pos(f32),
                Both(ColorProperty, f32),
            }

            let mut pieces: Vec<StopPiece> = Vec::new();
            loop {
                if let Ok(p) = parser.try_parse(|p| p.expect_percentage()) {
                    pieces.push(StopPiece::Pos(p));
                } else {
                    let color = next_color(parser)?;
                    let pos1 = parser.try_parse(|p| p.expect_percentage()).ok();
                    let pos2 = pos1.as_ref().and_then(|_| parser.try_parse(|p| p.expect_percentage()).ok());

                    match pos1 {
                        Some(pos1) => pieces.push(StopPiece::Both(color, pos1)),
                        None => pieces.push(StopPiece::Color(color)),
                    }
                    if let Some(pos2) = pos2 {
                        pieces.push(StopPiece::Both(color, pos2));
                    }
                }

                if parser.try_parse(|p| p.expect_comma()).is_err() {
                    break;
                }
            }

            parser.expect_exhausted()?;

            // Resolve final color stops
            let mut color_stop_count = 0usize;
            let mut first: Option<usize> = None;
            let mut last: usize = 0;

            for (idx, p) in pieces.iter().enumerate() {
                match p {
                    StopPiece::Pos(_) => {
                        if first.is_none() {
                            return Err(parser.new_error(BasicParseErrorKind::QualifiedRuleInvalid));
                        }
                    }
                    StopPiece::Color(_) | StopPiece::Both(_, _) => {
                        color_stop_count += 1;
                        if first.is_none() {
                            first = Some(idx);
                        }
                        last = idx;
                    }
                }
            }
            let first = first.ok_or_else(|| parser.new_error(BasicParseErrorKind::QualifiedRuleInvalid))?;
            if color_stop_count < 2 {
                return Err(parser.new_error(BasicParseErrorKind::QualifiedRuleInvalid));
            }

            let mut gradient_stops = Vec::with_capacity(pieces.len());

            // Normalize first missing pos to 0, last missing pos to 1
            match pieces[first] {
                StopPiece::Color(c) => pieces[first] = StopPiece::Both(c, 0.0),
                StopPiece::Both(_, _) => {}
                _ => return Err(parser.new_error(BasicParseErrorKind::QualifiedRuleInvalid)),
            }
            match pieces[last] {
                StopPiece::Color(c) => pieces[last] = StopPiece::Both(c, 1.0),
                StopPiece::Both(_, _) => {}
                _ => return Err(parser.new_error(BasicParseErrorKind::QualifiedRuleInvalid)),
            }

            let resolve_hint = |prev_color: ColorProperty, prev_pos: f32, next_color: ColorProperty, next_pos: f32, hint_raw: f32| -> (ColorProperty, f32) {
                let mut hpos = hint_raw;
                if hpos < prev_pos {
                    hpos = prev_pos;
                }
                if hpos > next_pos {
                    hpos = next_pos;
                }

                let hc = match (prev_color, next_color) {
                    (ColorProperty::CurrentColor, _) | (_, ColorProperty::CurrentColor) => ColorProperty::CurrentColor,
                    (ColorProperty::Color(a), ColorProperty::Color(b)) => {
                        let denom = next_pos - prev_pos;
                        let t = if denom.abs() > f32::EPSILON {
                            ((hpos - prev_pos) / denom).clamp(0.0, 1.0)
                        } else {
                            0.5
                        };

                        let da: DynamicColor = a.into();
                        let db: DynamicColor = b.into();
                        let interp = da.interpolate(db, interpolation_cs, hue_direction);
                        let out: DynamicColor = interp.eval(t);
                        let out_srgb: AlphaColor<Srgb> = out.to_alpha_color();
                        ColorProperty::Color(out_srgb)
                    }
                };

                (hc, hpos)
            };

            // Start from first definite stop
            let (mut last_color, mut last_pos) = match pieces[first] {
                StopPiece::Both(c, p) => (c, p),
                _ => return Err(parser.new_error(BasicParseErrorKind::QualifiedRuleInvalid)),
            };

            let mut i = first + 1;
            while i <= last {
                // Scan forward to find next definite endpoint and count missing colors
                let mut k = i;
                let mut missing = 0usize;
                loop {
                    if k > last {
                        return Err(parser.new_error(BasicParseErrorKind::QualifiedRuleInvalid)); // no leading hints
                    }
                    match pieces[k] {
                        StopPiece::Pos(_) => {}
                        StopPiece::Color(_) => missing += 1,
                        StopPiece::Both(_, _) => break,
                    }
                    k += 1;
                }

                // Clamp endpoint to be monotonic
                let (end_color, end_pos_raw) = match pieces[k] {
                    StopPiece::Both(c, p) => (c, p),
                    _ => return Err(parser.new_error(BasicParseErrorKind::QualifiedRuleInvalid)),
                };
                let end_pos = if end_pos_raw < last_pos { last_pos } else { end_pos_raw };
                pieces[k] = StopPiece::Both(end_color, end_pos);

                let slots = missing + 1;
                let step = (end_pos - last_pos) / (slots as f32);

                // Assign missing positions and resolve hints
                let mut assign = 0usize;
                let mut prev_color = last_color;
                let mut prev_pos = last_pos;
                let mut pending_hint: Option<(usize, f32)> = None;

                for j in i..=k {
                    match pieces[j] {
                        StopPiece::Pos(h) => {
                            if pending_hint.is_some() {
                                return Err(parser.new_error(BasicParseErrorKind::QualifiedRuleInvalid)); // consecutive hints
                            }
                            pending_hint = Some((j, h));
                        }
                        StopPiece::Color(c) => {
                            assign += 1;
                            let pos = last_pos + step * (assign as f32);
                            pieces[j] = StopPiece::Both(c, pos);

                            if let Some((hidx, hraw)) = pending_hint.take() {
                                let (hc, hpos) = resolve_hint(prev_color, prev_pos, c, pos, hraw);
                                pieces[hidx] = StopPiece::Both(hc, hpos);
                            }
                            prev_color = c;
                            prev_pos = pos;
                        }
                        StopPiece::Both(c, pos) => {
                            if j != k {
                                return Err(parser.new_error(BasicParseErrorKind::QualifiedRuleInvalid)); // unexpected Both inside segment
                            }
                            if let Some((hidx, hraw)) = pending_hint.take() {
                                let (hc, hpos) = resolve_hint(prev_color, prev_pos, c, pos, hraw);
                                pieces[hidx] = StopPiece::Both(hc, hpos);
                            }
                            prev_color = c;
                            prev_pos = pos;
                        }
                    }
                }

                if pending_hint.is_some() {
                    return Err(parser.new_error(BasicParseErrorKind::QualifiedRuleInvalid)); // hint with no right endpoint
                }

                last_color = end_color;
                last_pos = end_pos;
                i = k + 1;
            }

            // Emit final stops
            for p in pieces.into_iter() {
                match p {
                    StopPiece::Both(c, pos) => gradient_stops.push((pos, c)),
                    _ => return Err(parser.new_error(BasicParseErrorKind::QualifiedRuleInvalid)),
                }
            }

            Ok(LinearGradient {
                angle,
                gradient_stops,
                interpolation_cs,
                hue_direction,
            })
        })
    })?;

    Ok(PropertyValue::Exact(GradientStack { stack: Arc::new(gradients) }))
}

pub(crate) fn parse_border_sh<'i>(parser: &mut Parser<'i, '_>) -> Result<Props, cssparser::ParseError<'i, CustomParseError>> {
    let mut result = Props::new();

    if is_keyword_exhausted(parser, "initial") {
        push_property_array(BORDER_COLORS, PropertyValue::Initial, &mut result);
        push_property_array(BORDER_WIDTHS, PropertyValue::Initial, &mut result);
        return Ok(result);
    }
    if is_keyword_exhausted(parser, "inherit") {
        push_property_array(BORDER_COLORS, PropertyValue::Inherit, &mut result);
        push_property_array(BORDER_WIDTHS, PropertyValue::Inherit, &mut result);
        return Ok(result);
    }

    let mut seen_color = false;
    let mut seen_width = false;
    let mut seen_style = false;

    while !parser.is_exhausted() {
        match parser.try_parse(|p| next_color(p)) {
            Ok(color) => {
                if seen_color {
                    return Err(parser.new_error_for_next_token());
                }
                seen_color = true;
                push_property_array(BORDER_COLORS, PropertyValue::Exact(color), &mut result);
                continue;
            }
            Err(e) => {
                if cssparser::ParseErrorKind::Custom(CustomParseError::VarFunction) == e.kind {
                    return Err(e);
                }
            }
        }

        if let Ok(val) = parser.try_parse(|p| {
            if let Ok(ident) = p.try_parse(|p2| p2.expect_ident().cloned()) {
                return Ok(match_ignore_ascii_case! { &ident,
                    "thin" => Length::Px(2.0),
                    "medium" => Length::Px(4.0),
                    "thick" => Length::Px(6.0),
                    _ => {
                        return Err(p.new_error_for_next_token());
                    }
                });
            }
            next_positive_length(p)
        }) {
            if seen_width {
                return Err(parser.new_error_for_next_token());
            }
            seen_width = true;
            push_property_array(BORDER_WIDTHS, PropertyValue::Exact(val), &mut result);
            continue;
        }

        match parser.next()? {
            Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                return Err(parser.new_custom_error(CustomParseError::VarFunction));
            }
            Token::Ident(s) => match_ignore_ascii_case! { s,
                "solid" => {
                    if seen_style {
                        return Err(parser.new_error_for_next_token());
                    }
                    seen_style = true;
                },
                "dotted" | "dashed" | "double" | "groove" | "ridge" | "inset" | "outset" => {
                    return Err(parser.new_custom_error(CustomParseError::UnsupportedValue));
                },
                _ => return Err(parser.new_error_for_next_token()),
            },
            _ => return Err(parser.new_error_for_next_token()),
        }
    }

    Ok(result)
}

pub(crate) fn parse_border_color_sh<'i>(parser: &mut Parser<'i, '_>) -> Result<Props, cssparser::ParseError<'i, CustomParseError>> {
    let mut result = Props::new();

    if is_keyword_exhausted(parser, "initial") {
        push_property_array(BORDER_COLORS, PropertyValue::Initial, &mut result);
        return Ok(result);
    }
    if is_keyword_exhausted(parser, "inherit") {
        push_property_array(BORDER_COLORS, PropertyValue::Inherit, &mut result);
        return Ok(result);
    }

    let [top, right, bottom, left] = quad_values(parser, parse_color)?;
    result.push(Property::BorderTopColor(top));
    result.push(Property::BorderRightColor(right));
    result.push(Property::BorderBottomColor(bottom));
    result.push(Property::BorderLeftColor(left));
    Ok(result)
}

pub(crate) fn parse_border_radius_sh<'i>(parser: &mut Parser<'i, '_>) -> Result<Props, cssparser::ParseError<'i, CustomParseError>> {
    let mut result = Props::with_capacity(4);

    if is_keyword_exhausted(parser, "initial") {
        push_property_array(BORDER_RADII, PropertyValue::Initial, &mut result);
        return Ok(result);
    }
    if is_keyword_exhausted(parser, "inherit") {
        push_property_array(BORDER_RADII, PropertyValue::Inherit, &mut result);
        return Ok(result);
    }

    let [tl, tr, br, bl] = quad_values(parser, |p| Ok(PropertyValue::Exact(next_positive_length(p)?)))?;

    result.push(Property::BorderTopLeftRadius(tl));
    result.push(Property::BorderTopRightRadius(tr));
    result.push(Property::BorderBottomRightRadius(br));
    result.push(Property::BorderBottomLeftRadius(bl));
    Ok(result)
}

pub(crate) fn parse_border_side_sh<'i>(parser: &mut Parser<'i, '_>, side: BorderSide) -> Result<Props, cssparser::ParseError<'i, CustomParseError>> {
    let mut result = Props::new();

    type ColorProp = fn(PropertyValue<ColorProperty>) -> Property;
    type WidthProp = fn(PropertyValue<Length>) -> Property;

    let (color_prop, width_prop): (ColorProp, WidthProp) = match side {
        BorderSide::Top => (Property::BorderTopColor, Property::BorderTopWidth),
        BorderSide::Right => (Property::BorderRightColor, Property::BorderRightWidth),
        BorderSide::Bottom => (Property::BorderBottomColor, Property::BorderBottomWidth),
        BorderSide::Left => (Property::BorderLeftColor, Property::BorderLeftWidth),
    };

    if is_keyword_exhausted(parser, "initial") {
        result.push(color_prop(PropertyValue::Initial));
        result.push(width_prop(PropertyValue::Initial));
        return Ok(result);
    }
    if is_keyword_exhausted(parser, "inherit") {
        result.push(color_prop(PropertyValue::Inherit));
        result.push(width_prop(PropertyValue::Inherit));
        return Ok(result);
    }

    stroke_components(parser, &mut result, color_prop, width_prop)?;
    Ok(result)
}

pub(crate) fn parse_border_width_sh<'i>(parser: &mut Parser<'i, '_>) -> Result<Props, cssparser::ParseError<'i, CustomParseError>> {
    let mut result = Props::new();

    if is_keyword_exhausted(parser, "initial") {
        push_property_array(BORDER_WIDTHS, PropertyValue::Initial, &mut result);
        return Ok(result);
    }

    if is_keyword_exhausted(parser, "inherit") {
        push_property_array(BORDER_WIDTHS, PropertyValue::Inherit, &mut result);
        return Ok(result);
    }

    let [top, right, bottom, left] = quad_values(parser, |p| {
        if let Ok(ident) = p.try_parse(|p2| p2.expect_ident().cloned()) {
            return Ok(match_ignore_ascii_case! { &ident,
                "thin" => PropertyValue::Exact(Length::Px(2.0)),
                "medium" => PropertyValue::Exact(Length::Px(4.0)),
                "thick" => PropertyValue::Exact(Length::Px(6.0)),
                _ => return Err(p.new_error_for_next_token()),
            });
        }

        Ok(PropertyValue::Exact(next_positive_length(p)?))
    })?;

    result.push(Property::BorderTopWidth(top));
    result.push(Property::BorderRightWidth(right));
    result.push(Property::BorderBottomWidth(bottom));
    result.push(Property::BorderLeftWidth(left));
    Ok(result)
}

pub(crate) fn parse_box_shadow<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<Arc<[BoxShadow]>>, cssparser::ParseError<'i, CustomParseError>> {
    shadow_list(parser, true, |color, inset, offset_x, offset_y, blur, spread| {
        let mut box_shadow = BoxShadow {
            color,
            inset,
            ..Default::default()
        };

        box_shadow.offset_x = offset_x;
        box_shadow.offset_y = offset_y;

        if let Some(b) = blur {
            box_shadow.blur = b;
        }
        if let Some(s) = spread {
            box_shadow.spread = s;
        }

        Ok(box_shadow)
    })
}

pub(crate) fn parse_display<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<Option<Direction>>, cssparser::ParseError<'i, CustomParseError>> {
    match parser.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(parser.new_custom_error(CustomParseError::VarFunction)),
        Token::Ident(s) => Ok(match_ignore_ascii_case! { s,
            "row" => PropertyValue::Exact(Some(Direction::Row)),
            "row-reverse" => PropertyValue::Exact(Some(Direction::RowReverse)),
            "column" => PropertyValue::Exact(Some(Direction::Column)),
            "column-reverse" => PropertyValue::Exact(Some(Direction::ColumnReverse)),
            "none" => PropertyValue::Exact(None),
            _ => return Err(parser.new_error_for_next_token()),
        }),
        _ => Err(parser.new_error_for_next_token()),
    }
}

pub(crate) fn parse_font_sh<'i>(parser: &mut Parser<'i, '_>) -> Result<Props, cssparser::ParseError<'i, CustomParseError>> {
    if is_keyword_exhausted(parser, "initial") {
        return Ok(smallvec![
            Property::FontFamily(PropertyValue::Initial),
            Property::FontStyle(PropertyValue::Initial),
            Property::FontWeight(PropertyValue::Initial),
            Property::FontWidth(PropertyValue::Initial),
            Property::FontSize(PropertyValue::Initial),
            Property::LineHeight(PropertyValue::Initial),
        ]);
    }

    if is_keyword_exhausted(parser, "inherit") {
        return Ok(smallvec![
            Property::FontFamily(PropertyValue::Inherit),
            Property::FontStyle(PropertyValue::Inherit),
            Property::FontWeight(PropertyValue::Inherit),
            Property::FontWidth(PropertyValue::Inherit),
            Property::FontSize(PropertyValue::Inherit),
            Property::LineHeight(PropertyValue::Inherit),
        ]);
    }

    let mut style: Option<PropertyValue<parley::style::FontStyle>> = None;
    let mut weight: Option<PropertyValue<f32>> = None;
    let mut width: Option<PropertyValue<f32>> = None;

    loop {
        if style.is_none()
            && let Ok(v) = parser.try_parse(|p| parse_font_style(p))
        {
            style = Some(v);
            continue;
        }
        if weight.is_none()
            && let Ok(v) = parser.try_parse(|p| parse_font_weight(p))
        {
            weight = Some(v);
            continue;
        }
        if width.is_none()
            && let Ok(v) = parser.try_parse(|p| parse_font_width(p))
        {
            width = Some(v);
            continue;
        }
        break;
    }

    let size = parse_font_size(parser)?;

    let line_height = if parser.try_parse(|p| p.expect_delim('/')).is_ok() {
        if parser.try_parse(|p| p.expect_ident_matching("normal")).is_ok() {
            PropertyValue::Initial
        } else {
            parse_unit(parser)?
        }
    } else {
        PropertyValue::Initial
    };

    let family = parse_font_family(parser)?;

    Ok(smallvec![
        Property::FontStyle(style.unwrap_or(PropertyValue::Initial)),
        Property::FontWeight(weight.unwrap_or(PropertyValue::Initial)),
        Property::FontWidth(width.unwrap_or(PropertyValue::Initial)),
        Property::FontSize(size),
        Property::LineHeight(line_height),
        Property::FontFamily(family),
    ])
}

pub(crate) fn parse_font_family<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<Arc<str>>, cssparser::ParseError<'i, CustomParseError>> {
    let start = parser.position();
    let mut is_first_family = true;

    while !parser.is_exhausted() {
        if !is_first_family {
            parser.expect_comma()?;

            if parser.is_exhausted() {
                return Err(parser.new_error(BasicParseErrorKind::EndOfInput));
            }
        }

        // Parse one family
        match parser.next()? {
            Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                return Err(parser.new_custom_error(CustomParseError::VarFunction));
            }
            Token::QuotedString(_) => {}
            Token::Ident(_) => loop {
                let st = parser.state();
                parser.skip_whitespace();

                match parser.next() {
                    Ok(Token::Ident(_)) => {}
                    Ok(Token::Function(name)) if name.eq_ignore_ascii_case("var") => {
                        return Err(parser.new_custom_error(CustomParseError::VarFunction));
                    }
                    Ok(Token::Comma) => {
                        parser.reset(&st);
                        break;
                    }
                    Ok(_) => {
                        return Err(parser.new_error_for_next_token());
                    }
                    Err(_) => break,
                }
            },
            _ => return Err(parser.new_error_for_next_token()),
        }

        is_first_family = false;
        parser.skip_whitespace();
    }

    if is_first_family {
        return Err(parser.new_error(BasicParseErrorKind::EndOfInput));
    }

    // Store the raw value after validating
    let raw = Arc::from(parser.slice_from(start).trim());

    Ok(PropertyValue::Exact(raw))
}

pub(crate) fn parse_font_size<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<f32>, cssparser::ParseError<'i, CustomParseError>> {
    match parser.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(parser.new_custom_error(CustomParseError::VarFunction)),
        Token::Number { value, .. } => {
            if *value < 0.0 {
                return Err(parser.new_custom_error(CustomParseError::InvalidValue));
            }
            Ok(PropertyValue::Exact(*value))
        }
        Token::Dimension { value, unit, .. } => {
            if unit.eq_ignore_ascii_case("px") {
                if *value < 0.0 {
                    return Err(parser.new_custom_error(CustomParseError::InvalidValue));
                }
                Ok(PropertyValue::Exact(*value))
            } else {
                Err(parser.new_custom_error(CustomParseError::UnsupportedValue))
            }
        }
        _ => Err(parser.new_error_for_next_token()),
    }
}

pub(crate) fn parse_font_style<'i>(
    parser: &mut Parser<'i, '_>,
) -> Result<PropertyValue<parley::style::FontStyle>, cssparser::ParseError<'i, CustomParseError>> {
    match parser.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(parser.new_custom_error(CustomParseError::VarFunction)),
        Token::Ident(s) => match_ignore_ascii_case! { s,
            "normal" => Ok(PropertyValue::Exact(parley::style::FontStyle::Normal)),
            "italic" => Ok(PropertyValue::Exact(parley::style::FontStyle::Italic)),
            "oblique" => {
                let angle = parser.try_parse(|p| match p.next()? {
                    Token::Dimension { value, unit, .. } if unit.eq_ignore_ascii_case("deg") => Ok(*value),
                    Token::Dimension { .. } => Err(p.new_custom_error(CustomParseError::UnsupportedValue)),
                    Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(p.new_custom_error(CustomParseError::VarFunction)),
                    _ => Err(p.new_error_for_next_token::<CustomParseError>()),
                }).ok();
                Ok(PropertyValue::Exact(parley::style::FontStyle::Oblique(angle)))
            },
            _ => Err(parser.new_error_for_next_token()),
        },
        _ => Err(parser.new_error_for_next_token()),
    }
}

pub(crate) fn parse_font_weight<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<f32>, cssparser::ParseError<'i, CustomParseError>> {
    let token = parser.next()?;
    match token {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(parser.new_custom_error(CustomParseError::VarFunction)),
        Token::Ident(s) => match_ignore_ascii_case! { s,
            "normal" => Ok(PropertyValue::Exact(parley::fontique::FontWeight::NORMAL.value())),
            "bold" => Ok(PropertyValue::Exact(parley::fontique::FontWeight::BOLD.value())),
            "bolder" | "lighter" => Err(parser.new_custom_error(CustomParseError::UnsupportedValue)),
            _ => Err(parser.new_error_for_next_token()),
        },
        Token::Number { value, .. } => Ok(PropertyValue::Exact(parley::fontique::FontWeight::new(value.clamp(1.0, 1000.0)).value())),
        _ => Err(parser.new_error_for_next_token()),
    }
}

pub(crate) fn parse_font_width<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<f32>, cssparser::ParseError<'i, CustomParseError>> {
    match parser.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(parser.new_custom_error(CustomParseError::VarFunction)),
        Token::Percentage { unit_value, .. } => {
            if *unit_value < 0.0 {
                return Err(parser.new_custom_error(CustomParseError::InvalidValue));
            }
            Ok(PropertyValue::Exact(*unit_value))
        }
        Token::Ident(s) => Ok(PropertyValue::Exact(match_ignore_ascii_case! { s,
            "ultra-condensed" => 0.5,
            "extra-condensed" => 0.625,
            "condensed" => 0.75,
            "semi-condensed" => 0.875,
            "normal" => 1.0,
            "semi-expanded" => 1.125,
            "expanded" => 1.25,
            "extra-expanded" => 1.5,
            "ultra-expanded" => 2.0,
            _ => return Err(parser.new_error_for_next_token()),
        })),
        _ => Err(parser.new_error_for_next_token()),
    }
}

pub(crate) fn parse_i32<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<i32>, cssparser::ParseError<'i, CustomParseError>> {
    let value = PropertyValue::Exact(match parser.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(parser.new_custom_error(CustomParseError::VarFunction)),
        Token::Number { int_value: Some(v), .. } => Ok(*v),
        _ => Err(parser.new_error_for_next_token()),
    }?);
    Ok(value)
}

pub(crate) fn parse_outline_sh<'i>(parser: &mut Parser<'i, '_>) -> Result<Props, cssparser::ParseError<'i, CustomParseError>> {
    let mut result = Props::new();

    if is_keyword_exhausted(parser, "initial") {
        result.push(Property::OutlineColor(PropertyValue::Initial));
        result.push(Property::OutlineWidth(PropertyValue::Initial));
        return Ok(result);
    }

    if is_keyword_exhausted(parser, "inherit") {
        result.push(Property::OutlineColor(PropertyValue::Inherit));
        result.push(Property::OutlineWidth(PropertyValue::Inherit));
        return Ok(result);
    }

    stroke_components(parser, &mut result, Property::OutlineColor, Property::OutlineWidth)?;
    Ok(result)
}

pub(crate) fn parse_position<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<Position>, cssparser::ParseError<'i, CustomParseError>> {
    match parser.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(parser.new_custom_error(CustomParseError::VarFunction)),
        Token::Ident(s) => Ok(match_ignore_ascii_case! { s,
            "parent-directed" => PropertyValue::Exact(Position::ParentDirected),
            "self-directed" => PropertyValue::Exact(Position::SelfDirected),
            "fixed" => PropertyValue::Exact(Position::Fixed),
            _ => return Err(parser.new_error_for_next_token()),
        }),
        _ => Err(parser.new_error_for_next_token()),
    }
}

pub(crate) fn parse_space_sh<'i>(parser: &mut Parser<'i, '_>) -> Result<Props, cssparser::ParseError<'i, CustomParseError>> {
    let mut result = Props::with_capacity(4);

    if is_keyword_exhausted(parser, "initial") {
        push_property_array(SPACE_PROPS, PropertyValue::Initial, &mut result);
        return Ok(result);
    }

    if is_keyword_exhausted(parser, "inherit") {
        push_property_array(SPACE_PROPS, PropertyValue::Inherit, &mut result);
        return Ok(result);
    }

    let [top, right, bottom, left] = quad_values(parser, parse_unit)?;
    result.push(Property::Top(top));
    result.push(Property::Right(right));
    result.push(Property::Bottom(bottom));
    result.push(Property::Left(left));

    Ok(result)
}

pub(crate) fn parse_child_space_sh<'i>(parser: &mut Parser<'i, '_>) -> Result<Props, cssparser::ParseError<'i, CustomParseError>> {
    let mut result = Props::with_capacity(4);

    if is_keyword_exhausted(parser, "initial") {
        push_property_array(CHILD_SPACE_PROPS, PropertyValue::Initial, &mut result);
        return Ok(result);
    }

    if is_keyword_exhausted(parser, "inherit") {
        push_property_array(CHILD_SPACE_PROPS, PropertyValue::Inherit, &mut result);
        return Ok(result);
    }

    let [top, right, bottom, left] = quad_values(parser, parse_unit)?;
    result.push(Property::ChildTop(top));
    result.push(Property::ChildRight(right));
    result.push(Property::ChildBottom(bottom));
    result.push(Property::ChildLeft(left));

    Ok(result)
}

pub(crate) fn parse_text_align<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<TextAlign>, cssparser::ParseError<'i, CustomParseError>> {
    match parser.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(parser.new_custom_error(CustomParseError::VarFunction)),
        Token::Ident(s) => Ok(PropertyValue::Exact(match_ignore_ascii_case! { s,
            "start" => TextAlign::Start,
            "end" => TextAlign::End,
            "left" => TextAlign::Left,
            "right" => TextAlign::Right,
            "center" => TextAlign::Center,
            "justify" => TextAlign::Justify,
            _ => return Err(parser.new_error_for_next_token()),
        })),
        _ => Err(parser.new_error_for_next_token()),
    }
}

pub(crate) fn parse_text_shadow<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<Arc<[TextShadow]>>, cssparser::ParseError<'i, CustomParseError>> {
    shadow_list(parser, false, |color, _inset, offset_x, offset_y, blur, spread| {
        // Text-shadow doesn't have spread.
        if spread.is_some() {
            return Err(());
        }

        let mut text_shadow = TextShadow { color, ..Default::default() };

        text_shadow.offset_x = offset_x;
        text_shadow.offset_y = offset_y;

        if let Some(b) = blur {
            text_shadow.blur = b;
        }

        Ok(text_shadow)
    })
}

pub(crate) fn parse_transform<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<Affine>, ParseError<'i, CustomParseError>> {
    if parser
        .try_parse(|p| {
            p.expect_ident_matching("none")?;
            p.expect_exhausted()
        })
        .is_ok()
    {
        return Ok(PropertyValue::Exact(Affine::IDENTITY));
    }

    let mut result = Affine::IDENTITY.as_coeffs();

    while !parser.is_exhausted() {
        let token = parser.next()?.clone();

        let func = match token {
            Token::Function(name) => {
                if name.eq_ignore_ascii_case("var") {
                    return Err(parser.new_custom_error(CustomParseError::VarFunction));
                }
                name
            }
            Token::Ident(ident) if ident.eq_ignore_ascii_case("none") => {
                return Err(parser.new_error(BasicParseErrorKind::UnexpectedToken(Token::Ident(ident))));
            }
            other => {
                return Err(parser.new_error(BasicParseErrorKind::UnexpectedToken(other)));
            }
        };

        let m: [f64; 6] = match_ignore_ascii_case! { &func,
            "translate" => {
                parser.parse_nested_block(|p| {
                    let tx = match p.next()? {
                        Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                            return Err(p.new_custom_error(CustomParseError::VarFunction));
                        }
                        Token::Number { value, .. } if *value == 0.0 => 0.0,
                        Token::Dimension { value, unit, .. } if unit.eq_ignore_ascii_case("px") => *value as f64,
                        Token::Dimension { .. } | Token::Percentage { .. } => {
                            return Err(p.new_custom_error(CustomParseError::UnsupportedValue));
                        }
                        _ => return Err(p.new_error_for_next_token()),
                    };

                    if p.is_exhausted() {
                        return Ok([1.0, 0.0, 0.0, 1.0, tx, 0.0]);
                    }

                    optional_comma(p);

                    let ty = match p.next()? {
                        Token::Function(name) if name.eq_ignore_ascii_case("var") => {
                            return Err(p.new_custom_error(CustomParseError::VarFunction));
                        }
                        Token::Number { value, .. } if *value == 0.0 => 0.0,
                        Token::Dimension { value, unit, .. } if unit.eq_ignore_ascii_case("px") => *value as f64,
                        Token::Dimension { .. } | Token::Percentage { .. } => {
                            return Err(p.new_custom_error(CustomParseError::UnsupportedValue));
                        }
                        _ => return Err(p.new_error_for_next_token()),
                    };

                    p.expect_exhausted()?;
                    Ok([1.0, 0.0, 0.0, 1.0, tx, ty])
                })?
            },
            "rotate" => {
                parser.parse_nested_block(|p| {
                    let theta = next_angle_radians(p)?;
                    p.expect_exhausted()?;
                    let (s, c) = theta.sin_cos();
                    Ok([c, s, -s, c, 0.0, 0.0])
                })?
            },
            "scale" => {
                parser.parse_nested_block(|p| {
                    let sx = next_number(p)?;
                    if p.is_exhausted() {
                        return Ok([sx, 0.0, 0.0, sx, 0.0, 0.0]);
                    }
                    optional_comma(p);
                    let sy = next_number(p)?;
                    p.expect_exhausted()?;
                    Ok([sx, 0.0, 0.0, sy, 0.0, 0.0])
                })?
            },
            "skew" => {
                parser.parse_nested_block(|p| {
                    let ax = next_angle_radians(p)?;
                    let ay = if p.is_exhausted() {
                        0.0
                    } else {
                        optional_comma(p);
                        next_angle_radians(p)?
                    };
                    p.expect_exhausted()?;

                    // x' = x + tan(ax)*y
                    // y' = tan(ay)*x + y
                    let tx = ax.tan();
                    let ty = ay.tan();
                    Ok([1.0, ty, tx, 1.0, 0.0, 0.0])
                })?
            },
            "matrix" => {
                parser.parse_nested_block(|p| {
                    let a = next_number(p)?;
                    optional_comma(p);

                    let b = next_number(p)?;
                    optional_comma(p);

                    let c = next_number(p)?;
                    optional_comma(p);

                    let d = next_number(p)?;
                    optional_comma(p);

                    let e = next_number(p)?;
                    optional_comma(p);

                    let f = next_number(p)?;
                    p.expect_exhausted()?;

                    Ok([a, b, c, d, e, f])
                })?
            },
            _ => return Err(parser.new_custom_error(CustomParseError::UnsupportedValue)),
        };

        let [a2, b2, c2, d2, e2, f2] = m;
        let [a1, b1, c1, d1, e1, f1] = result;

        result = [
            a2 * a1 + c2 * b1,
            b2 * a1 + d2 * b1,
            a2 * c1 + c2 * d1,
            b2 * c1 + d2 * d1,
            a2 * e1 + c2 * f1 + e2,
            b2 * e1 + d2 * f1 + f2,
        ];
    }

    Ok(PropertyValue::Exact(Affine::new(result)))
}

pub(crate) fn parse_unit<'i>(parser: &mut Parser<'i, '_>) -> Result<PropertyValue<Unit>, cssparser::ParseError<'i, CustomParseError>> {
    match parser.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => Err(parser.new_custom_error(CustomParseError::VarFunction))?,
        Token::Number { value, .. } => Ok(PropertyValue::Exact(Unit::Stretch(*value))),
        Token::Dimension { value, unit, .. } => match_ignore_ascii_case! { unit,
            "s"  => Ok(PropertyValue::Exact(Unit::Stretch(*value))),
            "px" => Ok(PropertyValue::Exact(Unit::Px(*value))),
            "em" => Ok(PropertyValue::Exact(Unit::Em(*value))),
            _    => Err(parser.new_error_for_next_token()),
        },
        Token::Percentage { unit_value, .. } => Ok(PropertyValue::Exact(Unit::Percent(*unit_value))),
        Token::Ident(s) => match_ignore_ascii_case! { s,
            "auto" => Ok(PropertyValue::Exact(Unit::Auto)),
            _      => Err(parser.new_error_for_next_token()),
        },
        _ => Err(parser.new_error_for_next_token()),
    }
}

pub(crate) fn parse_positive_unit<'i>(p: &mut Parser<'i, '_>) -> Result<PropertyValue<Unit>, ParseError<'i, CustomParseError>> {
    let u = match p.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("var") => {
            return Err(p.new_custom_error(CustomParseError::VarFunction));
        }
        Token::Number { value, .. } => Unit::Stretch(*value),
        Token::Dimension { value, unit, .. } => match_ignore_ascii_case! { unit,
            "s"  => Unit::Stretch(*value),
            "px" => Unit::Px(*value),
            "em" => Unit::Em(*value),
            _    => return Err(p.new_error_for_next_token()),
        },
        Token::Percentage { unit_value, .. } => Unit::Percent(*unit_value),
        Token::Ident(s) => match_ignore_ascii_case! { s,
            "auto" => Unit::Auto,
            _      => return Err(p.new_error_for_next_token()),
        },
        _ => return Err(p.new_error_for_next_token()),
    };

    let neg = match u {
        Unit::Px(x) | Unit::Em(x) | Unit::Stretch(x) | Unit::Percent(x) => x < 0.0,
        Unit::Auto => false,
    };

    if neg {
        return Err(p.new_custom_error(CustomParseError::InvalidValue));
    }

    Ok(PropertyValue::Exact(u))
}

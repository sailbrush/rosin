#![allow(clippy::cognitive_complexity)]

use crate::style::*;

use cssparser::*;

// TODO: if debug_assertions, print parse errors to stderr
// TODO: :focus, :hover, etc

macro_rules! parse {
    (@color, $parser:ident, $property_type:expr) => {
        Ok(vec![$property_type(PropertyValue::Exact(cssparser::Color::parse(
            $parser,
        )?))])
    };
    (@enum, $parser:ident, $property_type:expr, $enum:ident) => {{
        let token = $parser.next()?;
        match token {
            Token::Ident(s) => match_ignore_ascii_case! { &s,
                "auto" => Ok(vec![$property_type(PropertyValue::Auto)]),
                "initial" => Ok(vec![$property_type(PropertyValue::Initial)]),
                "inherit" => Ok(vec![$property_type(PropertyValue::Inherit)]),
                _ => {
                    if let Ok(value) = $enum::from_css_token(&s) {
                        Ok(vec![$property_type(PropertyValue::Exact(value))])
                    } else {
                        Err($parser.new_error_for_next_token())
                    }
                }
            },
            _ => Err($parser.new_error_for_next_token()),
        }
    }};
    (@length, $parser:ident, $property_type:expr) => {{
        let token = $parser.next()?;
        match token {
            Token::Number { .. } | Token::Dimension { .. } => Ok(vec![$property_type(PropertyValue::Exact(token.into()))]),
            Token::Ident(s) => match_ignore_ascii_case! { &s,
                "auto" => Ok(vec![$property_type(PropertyValue::Auto)]),
                "initial" => Ok(vec![$property_type(PropertyValue::Initial)]),
                "inherit" => Ok(vec![$property_type(PropertyValue::Inherit)]),
                _ => Err($parser.new_error_for_next_token()),
            },
            _ => Err($parser.new_error_for_next_token()),
        }
    }};
    (@i32, $parser:ident, $property_type:expr) => {{
        let token = $parser.next()?;
        match token {
            Token::Number { int_value, .. } => {
                if let Some(int_value) = *int_value {
                    Ok(vec![$property_type(PropertyValue::Exact(int_value))])
                } else {
                    Err($parser.new_error_for_next_token())
                }
            }
            Token::Ident(s) => match_ignore_ascii_case! { &s,
                "auto" => Ok(vec![$property_type(PropertyValue::Auto)]),
                "initial" => Ok(vec![$property_type(PropertyValue::Initial)]),
                "inherit" => Ok(vec![$property_type(PropertyValue::Inherit)]),
                _ => Err($parser.new_error_for_next_token()),
            },
            _ => Err($parser.new_error_for_next_token()),
        }
    }};
    (@u32, $parser:ident, $property_type:expr) => {{
        let token = $parser.next()?;
        match token {
            Token::Number { int_value, .. } => {
                if let Some(int_value) = *int_value {
                    Ok(vec![$property_type(PropertyValue::Exact(int_value as u32))])
                } else {
                    Err($parser.new_error_for_next_token())
                }
            }
            Token::Ident(s) => match_ignore_ascii_case! { &s,
                "auto" => Ok(vec![$property_type(PropertyValue::Auto)]),
                "initial" => Ok(vec![$property_type(PropertyValue::Initial)]),
                "inherit" => Ok(vec![$property_type(PropertyValue::Inherit)]),
                _ => Err($parser.new_error_for_next_token()),
            },
            _ => Err($parser.new_error_for_next_token()),
        }
    }};
    (@f32, $parser:ident, $property_type:expr) => {{
        let token = $parser.next()?;
        match token {
            Token::Number { value, .. } => Ok(vec![$property_type(PropertyValue::Exact(*value))]),
            Token::Ident(s) => match_ignore_ascii_case! { &s,
                "auto" => Ok(vec![$property_type(PropertyValue::Auto)]),
                "initial" => Ok(vec![$property_type(PropertyValue::Initial)]),
                "inherit" => Ok(vec![$property_type(PropertyValue::Inherit)]),
                _ => Err($parser.new_error_for_next_token()),
            },
            _ => Err($parser.new_error_for_next_token()),
        }
    }};
    (@quad, $parser:ident, $top:expr, $right:expr, $bottom:expr, $left:expr) => {{
        let mut result = Vec::new();
        let mut sizes: Vec<PropertyValue<Length>> = Vec::with_capacity(4);

        while !$parser.is_exhausted() {
            let token = $parser.next()?;
            match token {
                Token::Number { .. } | Token::Dimension { .. } => {
                    sizes.push(PropertyValue::Exact(token.into()));
                }
                Token::Ident(s) => match_ignore_ascii_case! { &s,
                    "auto" => sizes.push(PropertyValue::Auto),
                    "initial" => sizes.push(PropertyValue::Initial),
                    "inherit" => sizes.push(PropertyValue::Inherit),
                    _ => return Err($parser.new_error_for_next_token()),
                },
                _ => return Err($parser.new_error_for_next_token()),
            }
        }

        match sizes.len() {
            1 => {
                result.push($top(sizes[0]));
                result.push($right(sizes[0]));
                result.push($bottom(sizes[0]));
                result.push($left(sizes[0]));
            }
            2 => {
                result.push($top(sizes[0]));
                result.push($right(sizes[1]));
                result.push($bottom(sizes[0]));
                result.push($left(sizes[1]));
            }
            3 => {
                result.push($top(sizes[0]));
                result.push($right(sizes[1]));
                result.push($bottom(sizes[2]));
                result.push($left(sizes[1]));
            }
            4 => {
                result.push($top(sizes[0]));
                result.push($right(sizes[1]));
                result.push($bottom(sizes[2]));
                result.push($left(sizes[3]));
            }
            _ => return Err($parser.new_error_for_next_token()),
        }

        Ok(result)
    }};
}

#[derive(Debug, Copy, Clone)]
pub enum Length {
    Px(f32),
    Em(f32),
}

impl Default for Length {
    fn default() -> Self {
        Self::Px(0.0)
    }
}

impl From<&Token<'_>> for Length {
    fn from(token: &Token) -> Self {
        match token {
            Token::Number { value, .. } => Self::Px(*value as f32),
            Token::Dimension { value, unit, .. } => match unit.as_ref() {
                "em" => Self::Em(*value),
                _ => Self::Px(*value),
            },
            _ => panic!(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PropertyValue<T> {
    Auto,
    Initial,
    Inherit,
    Exact(T),
}

impl<T> Default for PropertyValue<T> {
    fn default() -> Self {
        PropertyValue::Initial
    }
}

impl<T> From<T> for PropertyValue<T> {
    fn from(value: T) -> Self {
        PropertyValue::Exact(value)
    }
}

#[derive(Debug, Clone)]
pub enum Property {
    AlignContent(PropertyValue<AlignContent>),
    AlignItems(PropertyValue<AlignItems>),
    AlignSelf(PropertyValue<AlignItems>),
    BackgroundColor(PropertyValue<cssparser::Color>),
    //TODO
    //BackgroundImage(PropertyValue<piet::FixedGradient>),
    BorderBottomColor(PropertyValue<cssparser::Color>),
    BorderBottomLeftRadius(PropertyValue<Length>),
    BorderBottomRightRadius(PropertyValue<Length>),
    BorderBottomWidth(PropertyValue<Length>),
    BorderLeftColor(PropertyValue<cssparser::Color>),
    BorderLeftWidth(PropertyValue<Length>),
    BorderRightColor(PropertyValue<cssparser::Color>),
    BorderRightWidth(PropertyValue<Length>),
    BorderTopColor(PropertyValue<cssparser::Color>),
    BorderTopLeftRadius(PropertyValue<Length>),
    BorderTopRightRadius(PropertyValue<Length>),
    BorderTopWidth(PropertyValue<Length>),
    Bottom(PropertyValue<Length>),
    BoxShadowOffsetX(PropertyValue<Length>),
    BoxShadowOffsetY(PropertyValue<Length>),
    BoxShadowBlur(PropertyValue<Length>),
    BoxShadowColor(PropertyValue<cssparser::Color>),
    BoxShadowInset(PropertyValue<bool>),
    Color(PropertyValue<cssparser::Color>),
    Cursor(PropertyValue<Cursor>),
    FlexBasis(PropertyValue<Length>),
    FlexDirection(PropertyValue<FlexDirection>),
    FlexGrow(PropertyValue<f32>),
    FlexShrink(PropertyValue<f32>),
    FlexWrap(PropertyValue<FlexWrap>),
    FontFamily(PropertyValue<usize>),
    FontSize(PropertyValue<Length>),
    FontWeight(PropertyValue<u32>),
    Height(PropertyValue<Length>),
    JustifyContent(PropertyValue<JustifyContent>),
    Left(PropertyValue<Length>),
    MarginBottom(PropertyValue<Length>),
    MarginLeft(PropertyValue<Length>),
    MarginRight(PropertyValue<Length>),
    MarginTop(PropertyValue<Length>),
    MaxHeight(PropertyValue<Length>),
    MaxWidth(PropertyValue<Length>),
    MinHeight(PropertyValue<Length>),
    MinWidth(PropertyValue<Length>),
    Opacity(PropertyValue<f32>),
    Order(PropertyValue<i32>),
    PaddingBottom(PropertyValue<Length>),
    PaddingLeft(PropertyValue<Length>),
    PaddingRight(PropertyValue<Length>),
    PaddingTop(PropertyValue<Length>),
    Position(PropertyValue<Position>),
    Right(PropertyValue<Length>),
    Top(PropertyValue<Length>),
    Width(PropertyValue<Length>),
    ZIndex(PropertyValue<i32>),
}

pub struct RulesParser;

impl<'i> AtRuleParser<'i> for RulesParser {
    type PreludeNoBlock = ();
    type PreludeBlock = ();
    type AtRule = Rule;
    type Error = ();
}

impl<'i> QualifiedRuleParser<'i> for RulesParser {
    type Prelude = (u32, Vec<Selector>);
    type QualifiedRule = Rule;
    type Error = ();

    fn parse_prelude<'t>(&mut self, parser: &mut Parser<'i, 't>) -> Result<Self::Prelude, ParseError<'i, Self::Error>> {
        let mut specificity = 0;
        let mut selector_list: Vec<Selector> = Vec::new();

        let mut first = true; // Is this the first identifier?
        let mut direct = false; // Has the `>` token been seen since last selector?
        let mut whitespace = false; // Has whitespace been seen since last selector?

        while !parser.is_exhausted() {
            match parser.next_including_whitespace()? {
                Token::Delim(c) => {
                    match c {
                        '*' => {
                            if !first && !direct && whitespace {
                                selector_list.push(Selector::Children);
                            }

                            selector_list.push(Selector::Wildcard);
                            whitespace = false;
                            direct = false;
                        }
                        '>' => {
                            selector_list.push(Selector::DirectChildren);
                            direct = true;
                        }
                        '.' => {}
                        _ => {
                            // TODO unexpected delim error? Or just ignore rule...
                        }
                    }
                }
                Token::Ident(s) => {
                    if !first && !direct && whitespace {
                        selector_list.push(Selector::Children);
                    }

                    selector_list.push(Selector::Class(s.to_string()));
                    specificity += 10;

                    whitespace = false;
                    direct = false;
                }
                Token::IDHash(s) | Token::Hash(s) => {
                    if !first && !direct && whitespace {
                        selector_list.push(Selector::Children);
                    }

                    selector_list.push(Selector::Id(s.to_string()));
                    specificity += 100;

                    whitespace = false;
                    direct = false;
                }
                Token::WhiteSpace(_) => {
                    whitespace = true;
                }
                _ => {
                    // TODO unexpected token error
                    return Err(parser.new_error_for_next_token());
                }
            }
            first = false;
        }
        Ok((specificity, selector_list))
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
        parser: &mut Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, ParseError<'i, Self::Error>> {
        let mut property_list = Vec::new();

        for result in DeclarationListParser::new(parser, PropertiesParser) {
            if let Ok(mut property) = result {
                property_list.append(&mut property);
            }
        }

        Ok(Rule {
            specificity: prelude.0,
            selectors: prelude.1,
            properties: property_list,
        })
    }
}

pub struct PropertiesParser;

impl<'i> AtRuleParser<'i> for PropertiesParser {
    type PreludeNoBlock = ();
    type PreludeBlock = ();
    type AtRule = Vec<Property>;
    type Error = ();
}

impl<'i> DeclarationParser<'i> for PropertiesParser {
    type Declaration = Vec<Property>;
    type Error = ();

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        parser: &mut Parser<'i, 't>,
    ) -> Result<Self::Declaration, ParseError<'i, Self::Error>> {
        match_ignore_ascii_case! { &name,
            "align-content" => parse!(@enum, parser, Property::AlignContent, AlignContent),
            "align-items" => parse!(@enum, parser, Property::AlignItems, AlignItems),
            "align-self" => parse!(@enum, parser, Property::AlignSelf, AlignItems),
            "background-color" => parse!(@color, parser, Property::BackgroundColor),
            "background-image" => {
                // Parse gradient function
                todo!()
            },
            "border" => {
                let mut result = Vec::new();

                while !parser.is_exhausted() {
                    let token = parser.next()?;
                    match token {
                        Token::Number { .. } | Token::Dimension { .. } => {
                            result.push(Property::BorderBottomWidth(PropertyValue::Exact(token.into())));
                            result.push(Property::BorderLeftWidth(PropertyValue::Exact(token.into())));
                            result.push(Property::BorderRightWidth(PropertyValue::Exact(token.into())));
                            result.push(Property::BorderTopWidth(PropertyValue::Exact(token.into())));
                        }
                        Token::Ident(s) => match_ignore_ascii_case! { &s,
                            "initial" => {
                                result.push(Property::BorderBottomColor(PropertyValue::Initial));
                                result.push(Property::BorderLeftColor(PropertyValue::Initial));
                                result.push(Property::BorderRightColor(PropertyValue::Initial));
                                result.push(Property::BorderTopColor(PropertyValue::Initial));
                            },
                            "inherit" => {
                                result.push(Property::BorderBottomColor(PropertyValue::Inherit));
                                result.push(Property::BorderLeftColor(PropertyValue::Inherit));
                                result.push(Property::BorderRightColor(PropertyValue::Inherit));
                                result.push(Property::BorderTopColor(PropertyValue::Inherit));
                            },
                            _ => {
                                let color = cssparser::Color::parse(parser)?;

                                result.push(Property::BorderBottomColor(PropertyValue::Exact(color)));
                                result.push(Property::BorderLeftColor(PropertyValue::Exact(color)));
                                result.push(Property::BorderRightColor(PropertyValue::Exact(color)));
                                result.push(Property::BorderTopColor(PropertyValue::Exact(color)));
                            },
                        },
                        _ => return Err(parser.new_error_for_next_token()),
                    }
                }

                Ok(result)
            },
            "border-bottom" => {
                let mut result = Vec::new();

                while !parser.is_exhausted() {
                    let token = parser.next()?;
                    match token {
                        Token::Number { .. } | Token::Dimension { .. } => {
                            result.push(Property::BorderBottomWidth(PropertyValue::Exact(token.into())));
                        }
                        Token::Ident(s) => match_ignore_ascii_case! { &s,
                            "initial" => result.push(Property::BorderBottomColor(PropertyValue::Initial)),
                            "inherit" => result.push(Property::BorderBottomColor(PropertyValue::Inherit)),
                            _ => {
                                let color = cssparser::Color::parse(parser)?;
                                result.push(Property::BorderBottomColor(PropertyValue::Exact(color)));
                            },
                        },
                        _ => return Err(parser.new_error_for_next_token()),
                    }
                }

                Ok(result)
            },
            "border-bottom-color" => parse!(@color, parser, Property::BorderBottomColor),
            "border-bottom-left-radius" => parse!(@length, parser, Property::BorderBottomLeftRadius),
            "border-bottom-right-radius" => parse!(@length, parser, Property::BorderBottomRightRadius),
            "border-bottom-width" => parse!(@length, parser, Property::BorderBottomWidth),
            "border-color" => {
                let mut result = Vec::new();
                let mut colors: Vec<PropertyValue<Color>> = Vec::with_capacity(4);

                while !parser.is_exhausted() {
                    let token = parser.next()?;
                    match token {
                        Token::Ident(s) => match_ignore_ascii_case! { &s,
                            "initial" => colors.push(PropertyValue::Initial),
                            "inherit" => colors.push(PropertyValue::Inherit),
                            _ => {
                                let color = cssparser::Color::parse(parser)?;
                                colors.push(PropertyValue::Exact(color));
                            },
                        },
                        _ => return Err(parser.new_error_for_next_token()),
                    }
                }

                match colors.len() {
                    1 => {
                        result.push(Property::BorderTopColor(colors[0]));
                        result.push(Property::BorderRightColor(colors[0]));
                        result.push(Property::BorderBottomColor(colors[0]));
                        result.push(Property::BorderLeftColor(colors[0]));
                    },
                    2 => {
                        result.push(Property::BorderTopColor(colors[0]));
                        result.push(Property::BorderRightColor(colors[1]));
                        result.push(Property::BorderBottomColor(colors[0]));
                        result.push(Property::BorderLeftColor(colors[1]));
                    },
                    3 => {
                        result.push(Property::BorderTopColor(colors[0]));
                        result.push(Property::BorderRightColor(colors[1]));
                        result.push(Property::BorderBottomColor(colors[2]));
                        result.push(Property::BorderLeftColor(colors[1]));
                    },
                    4 => {
                        result.push(Property::BorderTopColor(colors[0]));
                        result.push(Property::BorderRightColor(colors[1]));
                        result.push(Property::BorderBottomColor(colors[2]));
                        result.push(Property::BorderLeftColor(colors[3]));
                    },
                    _ => return Err(parser.new_error_for_next_token()),
                }

                Ok(result)
            },
            "border-left" => {
                let mut result = Vec::new();

                while !parser.is_exhausted() {
                    let token = parser.next()?;
                    match token {
                        Token::Number { .. } | Token::Dimension { .. } => {
                            result.push(Property::BorderLeftWidth(PropertyValue::Exact(token.into())));
                        }
                        Token::Ident(s) => match_ignore_ascii_case! { &s,
                            "initial" => result.push(Property::BorderLeftColor(PropertyValue::Initial)),
                            "inherit" => result.push(Property::BorderLeftColor(PropertyValue::Inherit)),
                            _ => {
                                let color = cssparser::Color::parse(parser)?;
                                result.push(Property::BorderLeftColor(PropertyValue::Exact(color)));
                            },
                        },
                        _ => return Err(parser.new_error_for_next_token()),
                    }
                }

                Ok(result)
            },
            "border-left-color" => parse!(@color, parser, Property::BorderLeftColor),
            "border-left-width" => parse!(@length, parser, Property::BorderLeftWidth),
            "border-radius" => parse!(@quad, parser, Property::BorderTopLeftRadius, Property::BorderTopRightRadius, Property::BorderBottomRightRadius, Property::BorderBottomLeftRadius),
            "border-right" => {
                let mut result = Vec::new();

                while !parser.is_exhausted() {
                    let token = parser.next()?;
                    match token {
                        Token::Number { .. } | Token::Dimension { .. } => {
                            result.push(Property::BorderRightWidth(PropertyValue::Exact(token.into())));
                        }
                        Token::Ident(s) => match_ignore_ascii_case! { &s,
                            "initial" => result.push(Property::BorderRightColor(PropertyValue::Initial)),
                            "inherit" => result.push(Property::BorderRightColor(PropertyValue::Inherit)),
                            _ => {
                                let color = cssparser::Color::parse(parser)?;
                                result.push(Property::BorderRightColor(PropertyValue::Exact(color)));
                            },
                        },
                        _ => return Err(parser.new_error_for_next_token()),
                    }
                }

                Ok(result)
            },
            "border-right-color" => parse!(@color, parser, Property::BorderRightColor),
            "border-right-width" => parse!(@length, parser, Property::BorderRightWidth),
            "border-top" => {
                let mut result = Vec::new();

                while !parser.is_exhausted() {
                    let token = parser.next()?;
                    match token {
                        Token::Number { .. } | Token::Dimension { .. } => {
                            result.push(Property::BorderTopWidth(PropertyValue::Exact(token.into())));
                        }
                        Token::Ident(s) => match_ignore_ascii_case! { &s,
                            "initial" => result.push(Property::BorderTopColor(PropertyValue::Initial)),
                            "inherit" => result.push(Property::BorderTopColor(PropertyValue::Inherit)),
                            _ => {
                                let color = cssparser::Color::parse(parser)?;
                                result.push(Property::BorderTopColor(PropertyValue::Exact(color)));
                            },
                        },
                        _ => return Err(parser.new_error_for_next_token()),
                    }
                }

                Ok(result)
            },
            "border-top-color" => parse!(@color, parser, Property::BorderTopColor),
            "border-top-left-radius" => parse!(@length, parser, Property::BorderTopLeftRadius),
            "border-top-right-radius" => parse!(@length, parser, Property::BorderTopRightRadius),
            "border-top-width" => parse!(@length, parser, Property::BorderTopWidth),
            "border-width" => {
                let mut result = Vec::new();
                let mut sizes: Vec<PropertyValue<Length>> = Vec::with_capacity(4);

                while !parser.is_exhausted() {
                    let token = parser.next()?;
                    match token {
                        Token::Number { .. } | Token::Dimension { .. } => {
                            sizes.push(PropertyValue::Exact(token.into()));
                        }
                        Token::Ident(s) => match_ignore_ascii_case! { &s,
                            "initial" => sizes.push(PropertyValue::Initial),
                            "inherit" => sizes.push(PropertyValue::Inherit),
                            "thin" => sizes.push(PropertyValue::Exact(Length::Px(2.0))),
                            "medium" => sizes.push(PropertyValue::Exact(Length::Px(4.0))),
                            "thick" => sizes.push(PropertyValue::Exact(Length::Px(6.0))),
                            _ => return Err(parser.new_error_for_next_token()),
                        },
                        _ => return Err(parser.new_error_for_next_token()),
                    }
                }

                match sizes.len() {
                    1 => {
                        result.push(Property::BorderTopWidth(sizes[0]));
                        result.push(Property::BorderRightWidth(sizes[0]));
                        result.push(Property::BorderBottomWidth(sizes[0]));
                        result.push(Property::BorderLeftWidth(sizes[0]));
                    },
                    2 => {
                        result.push(Property::BorderTopWidth(sizes[0]));
                        result.push(Property::BorderRightWidth(sizes[1]));
                        result.push(Property::BorderBottomWidth(sizes[0]));
                        result.push(Property::BorderLeftWidth(sizes[1]));
                    },
                    3 => {
                        result.push(Property::BorderTopWidth(sizes[0]));
                        result.push(Property::BorderRightWidth(sizes[1]));
                        result.push(Property::BorderBottomWidth(sizes[2]));
                        result.push(Property::BorderLeftWidth(sizes[1]));
                    },
                    4 => {
                        result.push(Property::BorderTopWidth(sizes[0]));
                        result.push(Property::BorderRightWidth(sizes[1]));
                        result.push(Property::BorderBottomWidth(sizes[2]));
                        result.push(Property::BorderLeftWidth(sizes[3]));
                    },
                    _ => return Err(parser.new_error_for_next_token()),
                }

                Ok(result)
            },
            "bottom" => parse!(@length, parser, Property::Bottom),
            "box-shadow" => todo!(),
            "color" => parse!(@color, parser, Property::Color),
            "cursor" => parse!(@enum, parser, Property::Cursor, Cursor),
            "flex" => {
                let mut result = Vec::new();

                while !parser.is_exhausted() {
                    let token = parser.next()?;
                    match token {
                        Token::Number { value, .. } => {
                            match result.len() {
                                0 => result.push(Property::FlexGrow(PropertyValue::Exact(*value))),
                                1 => result.push(Property::FlexShrink(PropertyValue::Exact(*value))),
                                2 => result.push(Property::FlexBasis(PropertyValue::Exact(Length::Px(*value)))),
                                _ => return Err(parser.new_error_for_next_token()),
                            }
                        },
                        Token::Dimension { unit, value, .. } => {
                            if result.len() == 2 {
                                match unit.as_ref() {
                                    "em" => result.push(Property::FlexBasis(PropertyValue::Exact(Length::Em(*value)))),
                                    _ => result.push(Property::FlexBasis(PropertyValue::Exact(Length::Px(*value)))),
                                };
                            } else {
                                return Err(parser.new_error_for_next_token());
                            }
                        },
                        Token::Ident(s) => match_ignore_ascii_case! { &s,
                            "auto" => {
                                result.push(Property::FlexGrow(PropertyValue::Exact(1.0)));
                                result.push(Property::FlexShrink(PropertyValue::Exact(1.0)));
                                result.push(Property::FlexBasis(PropertyValue::Auto));
                                break;
                            },
                            "none" => {
                                result.push(Property::FlexGrow(PropertyValue::Exact(0.0)));
                                result.push(Property::FlexShrink(PropertyValue::Exact(0.0)));
                                result.push(Property::FlexBasis(PropertyValue::Auto));
                                break;
                            },
                            "initial" => {
                                result.push(Property::FlexGrow(PropertyValue::Exact(0.0)));
                                result.push(Property::FlexShrink(PropertyValue::Exact(1.0)));
                                result.push(Property::FlexBasis(PropertyValue::Auto));
                                break;
                            },
                            "inherit" => {
                                result.push(Property::FlexGrow(PropertyValue::Inherit));
                                result.push(Property::FlexShrink(PropertyValue::Inherit));
                                result.push(Property::FlexBasis(PropertyValue::Inherit));
                                break;
                            },
                            _ => return Err(parser.new_error_for_next_token()),
                        },
                        _ => return Err(parser.new_error_for_next_token()),
                    }
                }

                Ok(result)
            },
            "flex-basis" => parse!(@length, parser, Property::FlexBasis),
            "flex-direction" => parse!(@enum, parser, Property::FlexDirection, FlexDirection),
            "flex-flow" => {
                let mut result = Vec::new();

                while !parser.is_exhausted() {
                    let token = parser.next()?;
                    match token {
                        Token::Ident(s) => match_ignore_ascii_case! { &s,
                            "initial" => {
                                result.push(Property::FlexDirection(PropertyValue::Initial));
                                result.push(Property::FlexWrap(PropertyValue::Initial));
                            },
                            "inherit" => {
                                result.push(Property::FlexDirection(PropertyValue::Inherit));
                                result.push(Property::FlexWrap(PropertyValue::Inherit));
                            },
                            _ => {
                                if let Ok(value) = FlexDirection::from_css_token(&s) {
                                    result.push(Property::FlexDirection(PropertyValue::Exact(value)));
                                } else if let Ok(value) = FlexWrap::from_css_token(&s) {
                                    result.push(Property::FlexWrap(PropertyValue::Exact(value)));
                                } else {
                                    return Err(parser.new_error_for_next_token())
                                }
                            },
                        },
                        _ => return Err(parser.new_error_for_next_token()),
                    }
                }

                Ok(result)
            },
            "flex-grow" => parse!(@f32, parser, Property::FlexGrow),
            "flex-shrink" => parse!(@f32, parser, Property::FlexShrink),
            "flex-wrap" => parse!(@enum, parser, Property::FlexWrap, FlexWrap),
            "font" => todo!(),
            "font-family" => todo!(),
            "font-size" => parse!(@length, parser, Property::FontSize),
            "font-weight" => parse!(@u32, parser, Property::FontWeight),
            "height" => parse!(@length, parser, Property::Height),
            "justify-content" => parse!(@enum, parser, Property::JustifyContent, JustifyContent),
            "left" => parse!(@length, parser, Property::Left),
            "margin" => parse!(@quad, parser, Property::MarginTop, Property::MarginRight, Property::MarginBottom, Property::MarginLeft),
            "margin-bottom" => parse!(@length, parser, Property::MarginBottom),
            "margin-left" => parse!(@length, parser, Property::MarginLeft),
            "margin-right" => parse!(@length, parser, Property::MarginRight),
            "margin-top" => parse!(@length, parser, Property::MarginTop),
            "max-height" => parse!(@length, parser, Property::MaxHeight),
            "max-width" => parse!(@length, parser, Property::MaxWidth),
            "min-height" => parse!(@length, parser, Property::MinHeight),
            "min-width" => parse!(@length, parser, Property::MinWidth),
            "opacity" => parse!(@f32, parser, Property::Opacity),
            "order" => parse!(@i32, parser, Property::Order),
            "padding" => parse!(@quad, parser, Property::PaddingTop, Property::PaddingRight, Property::PaddingBottom, Property::PaddingLeft),
            "padding-bottom" => parse!(@length, parser, Property::PaddingBottom),
            "padding-left" => parse!(@length, parser, Property::PaddingLeft),
            "padding-right" => parse!(@length, parser, Property::PaddingRight),
            "padding-top" => parse!(@length, parser, Property::PaddingTop),
            "position" => parse!(@enum, parser, Property::Position, Position),
            "right" => parse!(@length, parser, Property::Right),
            "top" => parse!(@length, parser, Property::Top),
            "width" => parse!(@length, parser, Property::Width),
            "z-index" => parse!(@i32, parser, Property::ZIndex),
            _ => Err(parser.new_error_for_next_token()),
        }
    }
}

use std::sync::Arc;

use crate::properties::*;
use crate::style::*;
use crate::stylesheet::*;

use cssparser::*;

// ---------- Rules Parser ----------

pub struct RulesParser;

impl<'i> AtRuleParser<'i> for RulesParser {
    type PreludeNoBlock = ();
    type PreludeBlock = ();
    type AtRule = (bool, Rule);
    type Error = ();
}

impl<'i> QualifiedRuleParser<'i> for RulesParser {
    type Prelude = (bool, u32, Vec<Selector>);
    type QualifiedRule = (bool, Rule);
    type Error = ();

    fn parse_prelude<'t>(&mut self, parser: &mut Parser<'i, 't>) -> Result<Self::Prelude, ParseError<'i, Self::Error>> {
        let mut specificity = 0;
        let mut dynamic = false; // Does this prelude include :hover or :focus selectors?
        let mut selector_list: Vec<Selector> = Vec::new();

        let mut first = true; // Is this the first identifier?
        let mut direct = false; // Has the `>` token been seen since last selector?
        let mut whitespace = false; // Has whitespace been seen since last selector?
        let mut colon = false; // Was previous token a colon?

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
                        _ => return Err(parser.new_error_for_next_token()),
                    }
                    colon = false;
                }
                Token::Ident(s) => {
                    if !first && !direct && !colon && whitespace {
                        selector_list.push(Selector::Children);
                    }

                    if colon {
                        match_ignore_ascii_case! { s,
                            "focus" => selector_list.push(Selector::Focus),
                            "hover" => selector_list.push(Selector::Hover),
                            _ => return Err(parser.new_error_for_next_token()),
                        }
                    } else {
                        selector_list.push(Selector::Class(s.to_string()));
                    }

                    specificity += 10;

                    whitespace = false;
                    direct = false;
                    colon = false;
                }
                Token::IDHash(s) | Token::Hash(s) => {
                    if !first && !direct && whitespace {
                        selector_list.push(Selector::Children);
                    }

                    selector_list.push(Selector::Id(s.to_string()));
                    specificity += 100;

                    whitespace = false;
                    direct = false;
                    colon = false;
                }
                Token::WhiteSpace(_) => {
                    whitespace = true;
                    colon = false;
                }
                Token::Colon => {
                    colon = true;
                    dynamic = true;
                }
                _ => return Err(parser.new_error_for_next_token()),
            }
            first = false;
        }
        Ok((dynamic, specificity, selector_list))
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
        parser: &mut Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, ParseError<'i, Self::Error>> {
        let mut property_list = Vec::new();

        for mut property in DeclarationListParser::new(parser, PropertiesParser).flatten() {
            property_list.append(&mut property);
        }

        Ok((
            prelude.0,
            Rule {
                specificity: prelude.1,
                selectors: prelude.2,
                properties: property_list,
            },
        ))
    }
}

// ---------- Properties Parser ----------

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
            "align-content" => parse_align_content(parser),
            "align-items" => parse_align_items(parser),
            "align-self" => parse_align_self(parser),
            "background-color" => Ok(vec![Property::BackgroundColor(PropertyValue::Exact(cssparser::Color::parse(parser)?))]),
            "background-image" => parse_background_image(parser),
            "border" => parse_border(parser),
            "border-bottom" => parse_border_bottom(parser),
            "border-bottom-color" => Ok(vec![Property::BorderBottomColor(PropertyValue::Exact(cssparser::Color::parse(parser)?))]),
            "border-bottom-left-radius" => Ok(vec![Property::BorderBottomLeftRadius(parse_length(parser)?)]),
            "border-bottom-right-radius" => Ok(vec![Property::BorderBottomRightRadius(parse_length(parser)?)]),
            "border-bottom-width" => Ok(vec![Property::BorderBottomWidth(parse_length(parser)?)]),
            "border-color" => parse_border_color(parser),
            "border-left" => parse_border_left(parser),
            "border-left-color" => Ok(vec![Property::BorderLeftColor(PropertyValue::Exact(cssparser::Color::parse(parser)?))]),
            "border-left-width" => Ok(vec![Property::BorderLeftWidth(parse_length(parser)?)]),
            "border-radius" => parse_border_radius(parser),
            "border-right" => parse_border_right(parser),
            "border-right-color" => Ok(vec![Property::BorderRightColor(PropertyValue::Exact(cssparser::Color::parse(parser)?))]),
            "border-right-width" => Ok(vec![Property::BorderRightWidth(parse_length(parser)?)]),
            "border-top" => parse_border_top(parser),
            "border-top-color" => Ok(vec![Property::BorderTopColor(PropertyValue::Exact(cssparser::Color::parse(parser)?))]),
            "border-top-left-radius" => Ok(vec![Property::BorderTopLeftRadius(parse_length(parser)?)]),
            "border-top-right-radius" => Ok(vec![Property::BorderTopRightRadius(parse_length(parser)?)]),
            "border-top-width" => Ok(vec![Property::BorderTopWidth(parse_length(parser)?)]),
            "border-width" => parse_border_width(parser),
            "bottom" => Ok(vec![Property::Bottom(parse_length(parser)?)]),
            "box-shadow" => todo!(),
            "color" => Ok(vec![Property::Color(PropertyValue::Exact(cssparser::Color::parse(parser)?))]),
            "cursor" => parse_cursor(parser),
            "flex" => parse_flex(parser),
            "flex-basis" => Ok(vec![Property::FlexBasis(parse_length(parser)?)]),
            "flex-direction" => parse_flex_direction(parser),
            "flex-flow" => parse_flex_flow(parser),
            "flex-grow" => Ok(vec![Property::FlexGrow(parse_f32(parser)?)]),
            "flex-shrink" => Ok(vec![Property::FlexShrink(parse_f32(parser)?)]),
            "flex-wrap" => parse_flex_wrap(parser),
            "font" => todo!(),
            "font-family" => parse_font_family(parser),
            "font-size" => Ok(vec![Property::FontSize(parse_length(parser)?)]),
            "font-weight" => Ok(vec![Property::FontWeight(parse_u32(parser)?)]),
            "height" => Ok(vec![Property::Height(parse_length(parser)?)]),
            "justify-content" => parse_justify_content(parser),
            "left" => Ok(vec![Property::Left(parse_length(parser)?)]),
            "margin" => parse_margin(parser),
            "margin-bottom" => Ok(vec![Property::MarginBottom(parse_length(parser)?)]),
            "margin-left" => Ok(vec![Property::MarginLeft(parse_length(parser)?)]),
            "margin-right" => Ok(vec![Property::MarginRight(parse_length(parser)?)]),
            "margin-top" => Ok(vec![Property::MarginTop(parse_length(parser)?)]),
            "max-height" => Ok(vec![Property::MaxHeight(parse_length(parser)?)]),
            "max-width" => Ok(vec![Property::MaxWidth(parse_length(parser)?)]),
            "min-height" => Ok(vec![Property::MinHeight(parse_length(parser)?)]),
            "min-width" => Ok(vec![Property::MinWidth(parse_length(parser)?)]),
            "opacity" => Ok(vec![Property::Opacity(parse_f32(parser)?)]),
            "order" => Ok(vec![Property::Order(parse_i32(parser)?)]),
            "padding" => parse_padding(parser),
            "padding-bottom" => Ok(vec![Property::PaddingBottom(parse_length(parser)?)]),
            "padding-left" => Ok(vec![Property::PaddingLeft(parse_length(parser)?)]),
            "padding-right" => Ok(vec![Property::PaddingRight(parse_length(parser)?)]),
            "padding-top" => Ok(vec![Property::PaddingTop(parse_length(parser)?)]),
            "position" => parse_position(parser),
            "right" => Ok(vec![Property::Right(parse_length(parser)?)]),
            "top" => Ok(vec![Property::Top(parse_length(parser)?)]),
            "width" => Ok(vec![Property::Width(parse_length(parser)?)]),
            "z-index" => Ok(vec![Property::ZIndex(parse_i32(parser)?)]),
            _ => Err(parser.new_error_for_next_token()),
        }
    }
}

// ---------- Parse Helper Funcitons ----------

fn parse_i32<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<PropertyValue<i32>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Number { int_value, .. } => {
            if let Some(int_value) = *int_value {
                Ok(PropertyValue::Exact(int_value))
            } else {
                Err(parser.new_error_for_next_token())
            }
        }
        Token::Ident(s) => match_ignore_ascii_case! { s,
            "auto" => Ok(PropertyValue::Auto),
            "initial" => Ok(PropertyValue::Initial),
            "inherit" => Ok(PropertyValue::Inherit),
            _ => Err(parser.new_error_for_next_token()),
        },
        _ => Err(parser.new_error_for_next_token()),
    }
}

fn parse_u32<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<PropertyValue<u32>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Number { int_value, .. } => {
            if let Some(int_value) = *int_value {
                Ok(PropertyValue::Exact(int_value as u32))
            } else {
                Err(parser.new_error_for_next_token())
            }
        }
        Token::Ident(s) => match_ignore_ascii_case! { s,
            "auto" => Ok(PropertyValue::Auto),
            "initial" => Ok(PropertyValue::Initial),
            "inherit" => Ok(PropertyValue::Inherit),
            _ => Err(parser.new_error_for_next_token()),
        },
        _ => Err(parser.new_error_for_next_token()),
    }
}

fn parse_f32<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<PropertyValue<f32>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Number { value, .. } => Ok(PropertyValue::Exact(*value)),
        Token::Ident(s) => match_ignore_ascii_case! { s,
            "auto" => Ok(PropertyValue::Auto),
            "initial" => Ok(PropertyValue::Initial),
            "inherit" => Ok(PropertyValue::Inherit),
            _ => Err(parser.new_error_for_next_token()),
        },
        _ => Err(parser.new_error_for_next_token()),
    }
}

fn parse_length<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<PropertyValue<Length>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Number { .. } | Token::Dimension { .. } => {
            if let Some(length) = parse_length_token(token) {
                Ok(PropertyValue::Exact(length))
            } else {
                Err(parser.new_error_for_next_token())
            }
        }
        Token::Ident(s) => match_ignore_ascii_case! { s,
            "auto" => Ok(PropertyValue::Auto),
            "initial" => Ok(PropertyValue::Initial),
            "inherit" => Ok(PropertyValue::Inherit),
            _ => Err(parser.new_error_for_next_token()),
        },
        _ => Err(parser.new_error_for_next_token()),
    }
}

fn parse_length_token(token: &Token) -> Option<Length> {
    match token {
        Token::Number { value, .. } => Some(Length::Px(*value as f32)),
        Token::Dimension { value, unit, .. } => match unit.as_ref() {
            "em" => Some(Length::Em(*value)),
            _ => Some(Length::Px(*value)),
        },
        _ => None,
    }
}

fn parse_quad<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<PropertyValue<Length>>, cssparser::ParseError<'i, ()>> {
    let mut sizes = Vec::with_capacity(4);

    while !parser.is_exhausted() {
        let token = parser.next()?;
        match token {
            Token::Number { .. } | Token::Dimension { .. } => {
                if let Some(length) = parse_length_token(token) {
                    sizes.push(PropertyValue::Exact(length));
                } else {
                    return Err(parser.new_error_for_next_token());
                }
            }
            Token::Ident(s) => match_ignore_ascii_case! { s,
                "auto" => sizes.push(PropertyValue::Auto),
                "initial" => sizes.push(PropertyValue::Initial),
                "inherit" => sizes.push(PropertyValue::Inherit),
                _ => return Err(parser.new_error_for_next_token()),
            },
            _ => return Err(parser.new_error_for_next_token()),
        }
    }

    Ok(sizes)
}

// ---------- Property Parsers ----------

fn parse_align_content<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Ident(s) => Ok(vec![Property::AlignContent(match_ignore_ascii_case! { s,
            "auto" => PropertyValue::Auto,
            "initial" => PropertyValue::Initial,
            "inherit" => PropertyValue::Inherit,
            "stretch" => PropertyValue::Exact(AlignContent::Stretch),
            "center" => PropertyValue::Exact(AlignContent::Center),
            "flex-start" => PropertyValue::Exact(AlignContent::FlexStart),
            "flex-end" => PropertyValue::Exact(AlignContent::FlexEnd),
            "space-between" => PropertyValue::Exact(AlignContent::SpaceBetween),
            "space-around" => PropertyValue::Exact(AlignContent::SpaceAround),
            _ => return Err(parser.new_error_for_next_token()),
        })]),
        _ => return Err(parser.new_error_for_next_token()),
    }
}

fn parse_align_items<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Ident(s) => Ok(vec![Property::AlignItems(match_ignore_ascii_case! { s,
            "auto" => PropertyValue::Auto,
            "initial" => PropertyValue::Initial,
            "inherit" => PropertyValue::Inherit,
            "stretch" => PropertyValue::Exact(AlignItems::Stretch),
            "center" => PropertyValue::Exact(AlignItems::Center),
            "flex-start" => PropertyValue::Exact(AlignItems::FlexStart),
            "flex-end" => PropertyValue::Exact(AlignItems::FlexEnd),
            _ => return Err(parser.new_error_for_next_token()),
        })]),
        _ => return Err(parser.new_error_for_next_token()),
    }
}

fn parse_align_self<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Ident(s) => Ok(vec![Property::AlignSelf(match_ignore_ascii_case! { s,
            "auto" => PropertyValue::Auto,
            "initial" => PropertyValue::Initial,
            "inherit" => PropertyValue::Inherit,
            "stretch" => PropertyValue::Exact(AlignItems::Stretch),
            "center" => PropertyValue::Exact(AlignItems::Center),
            "flex-start" => PropertyValue::Exact(AlignItems::FlexStart),
            "flex-end" => PropertyValue::Exact(AlignItems::FlexEnd),
            _ => return Err(parser.new_error_for_next_token()),
        })]),
        _ => return Err(parser.new_error_for_next_token()),
    }
}

fn parse_background_image<'i, 't>(_parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    // Parse gradient function
    todo!();
}

fn parse_border<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let mut result = Vec::new();

    while !parser.is_exhausted() {
        let token = parser.next()?;
        match token {
            Token::Number { .. } | Token::Dimension { .. } => {
                if let Some(length) = parse_length_token(token) {
                    result.push(Property::BorderBottomWidth(PropertyValue::Exact(length)));
                    result.push(Property::BorderLeftWidth(PropertyValue::Exact(length)));
                    result.push(Property::BorderRightWidth(PropertyValue::Exact(length)));
                    result.push(Property::BorderTopWidth(PropertyValue::Exact(length)));
                } else {
                    return Err(parser.new_error_for_next_token());
                }
            }
            Token::Ident(s) => match_ignore_ascii_case! { s,
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
}

fn parse_border_bottom<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let mut result = Vec::new();

    while !parser.is_exhausted() {
        let token = parser.next()?;
        match token {
            Token::Number { .. } | Token::Dimension { .. } => {
                if let Some(length) = parse_length_token(token) {
                    result.push(Property::BorderBottomWidth(PropertyValue::Exact(length)));
                } else {
                    return Err(parser.new_error_for_next_token());
                }
            }
            Token::Ident(s) => match_ignore_ascii_case! { s,
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
}

fn parse_border_color<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let mut result = Vec::new();
    let mut colors: Vec<PropertyValue<Color>> = Vec::with_capacity(4);

    while !parser.is_exhausted() {
        let token = parser.next()?;
        match token {
            Token::Ident(s) => match_ignore_ascii_case! { s,
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
        }
        2 => {
            result.push(Property::BorderTopColor(colors[0]));
            result.push(Property::BorderRightColor(colors[1]));
            result.push(Property::BorderBottomColor(colors[0]));
            result.push(Property::BorderLeftColor(colors[1]));
        }
        3 => {
            result.push(Property::BorderTopColor(colors[0]));
            result.push(Property::BorderRightColor(colors[1]));
            result.push(Property::BorderBottomColor(colors[2]));
            result.push(Property::BorderLeftColor(colors[1]));
        }
        4 => {
            result.push(Property::BorderTopColor(colors[0]));
            result.push(Property::BorderRightColor(colors[1]));
            result.push(Property::BorderBottomColor(colors[2]));
            result.push(Property::BorderLeftColor(colors[3]));
        }
        _ => return Err(parser.new_error_for_next_token()),
    }

    Ok(result)
}

fn parse_border_left<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let mut result = Vec::new();

    while !parser.is_exhausted() {
        let token = parser.next()?;
        match token {
            Token::Number { .. } | Token::Dimension { .. } => {
                if let Some(length) = parse_length_token(token) {
                    result.push(Property::BorderLeftWidth(PropertyValue::Exact(length)));
                } else {
                    return Err(parser.new_error_for_next_token());
                }
            }
            Token::Ident(s) => match_ignore_ascii_case! { s,
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
}

fn parse_border_right<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let mut result = Vec::new();

    while !parser.is_exhausted() {
        let token = parser.next()?;
        match token {
            Token::Number { .. } | Token::Dimension { .. } => {
                if let Some(length) = parse_length_token(token) {
                    result.push(Property::BorderRightWidth(PropertyValue::Exact(length)));
                } else {
                    return Err(parser.new_error_for_next_token());
                }
            }
            Token::Ident(s) => match_ignore_ascii_case! { s,
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
}

fn parse_border_top<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let mut result = Vec::new();

    while !parser.is_exhausted() {
        let token = parser.next()?;
        match token {
            Token::Number { .. } | Token::Dimension { .. } => {
                if let Some(length) = parse_length_token(token) {
                    result.push(Property::BorderTopWidth(PropertyValue::Exact(length)));
                } else {
                    return Err(parser.new_error_for_next_token());
                }
            }
            Token::Ident(s) => match_ignore_ascii_case! { s,
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
}

fn parse_border_radius<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let mut result = Vec::with_capacity(4);
    let sizes = parse_quad(parser)?;

    match sizes.len() {
        1 => {
            result.push(Property::BorderTopLeftRadius(sizes[0]));
            result.push(Property::BorderTopRightRadius(sizes[0]));
            result.push(Property::BorderBottomRightRadius(sizes[0]));
            result.push(Property::BorderBottomLeftRadius(sizes[0]));
        }
        2 => {
            result.push(Property::BorderTopLeftRadius(sizes[0]));
            result.push(Property::BorderTopRightRadius(sizes[1]));
            result.push(Property::BorderBottomRightRadius(sizes[0]));
            result.push(Property::BorderBottomLeftRadius(sizes[1]));
        }
        3 => {
            result.push(Property::BorderTopLeftRadius(sizes[0]));
            result.push(Property::BorderTopRightRadius(sizes[1]));
            result.push(Property::BorderBottomRightRadius(sizes[2]));
            result.push(Property::BorderBottomLeftRadius(sizes[1]));
        }
        4 => {
            result.push(Property::BorderTopLeftRadius(sizes[0]));
            result.push(Property::BorderTopRightRadius(sizes[1]));
            result.push(Property::BorderBottomRightRadius(sizes[2]));
            result.push(Property::BorderBottomLeftRadius(sizes[3]));
        }
        _ => return Err(parser.new_error_for_next_token()),
    }

    Ok(result)
}

fn parse_border_width<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let mut result = Vec::new();
    let mut sizes: Vec<PropertyValue<Length>> = Vec::with_capacity(4);

    while !parser.is_exhausted() {
        let token = parser.next()?;
        match token {
            Token::Number { .. } | Token::Dimension { .. } => {
                if let Some(length) = parse_length_token(token) {
                    sizes.push(PropertyValue::Exact(length));
                } else {
                    return Err(parser.new_error_for_next_token());
                }
            }
            Token::Ident(s) => match_ignore_ascii_case! { s,
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
        }
        2 => {
            result.push(Property::BorderTopWidth(sizes[0]));
            result.push(Property::BorderRightWidth(sizes[1]));
            result.push(Property::BorderBottomWidth(sizes[0]));
            result.push(Property::BorderLeftWidth(sizes[1]));
        }
        3 => {
            result.push(Property::BorderTopWidth(sizes[0]));
            result.push(Property::BorderRightWidth(sizes[1]));
            result.push(Property::BorderBottomWidth(sizes[2]));
            result.push(Property::BorderLeftWidth(sizes[1]));
        }
        4 => {
            result.push(Property::BorderTopWidth(sizes[0]));
            result.push(Property::BorderRightWidth(sizes[1]));
            result.push(Property::BorderBottomWidth(sizes[2]));
            result.push(Property::BorderLeftWidth(sizes[3]));
        }
        _ => return Err(parser.new_error_for_next_token()),
    }

    Ok(result)
}

fn parse_cursor<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Ident(s) => Ok(vec![Property::Cursor(match_ignore_ascii_case! { s,
            "auto" => PropertyValue::Auto,
            "initial" => PropertyValue::Initial,
            "inherit" => PropertyValue::Inherit,
            "default" => PropertyValue::Exact(Cursor::Default),
            "none" => PropertyValue::Exact(Cursor::None),
            "context-menu" => PropertyValue::Exact(Cursor::ContextMenu),
            "help" => PropertyValue::Exact(Cursor::Help),
            "pointer" => PropertyValue::Exact(Cursor::Pointer),
            "progress" => PropertyValue::Exact(Cursor::Progress),
            "wait" => PropertyValue::Exact(Cursor::Wait),
            "cell" => PropertyValue::Exact(Cursor::Cell),
            "crosshair" => PropertyValue::Exact(Cursor::Crosshair),
            "text" => PropertyValue::Exact(Cursor::Text),
            "vertical-text" => PropertyValue::Exact(Cursor::VerticalText),
            "alias" => PropertyValue::Exact(Cursor::Alias),
            "copy" => PropertyValue::Exact(Cursor::Copy),
            "move" => PropertyValue::Exact(Cursor::Move),
            "no-drop" => PropertyValue::Exact(Cursor::NoDrop),
            "not-allowed" => PropertyValue::Exact(Cursor::NotAllowed),
            "grab" => PropertyValue::Exact(Cursor::Grab),
            "grabbing" => PropertyValue::Exact(Cursor::Grabbing),
            "e-resize" => PropertyValue::Exact(Cursor::E_Resize),
            "n-resize" => PropertyValue::Exact(Cursor::N_Resize),
            "ne-resize" => PropertyValue::Exact(Cursor::NE_Resize),
            "nw-resize" => PropertyValue::Exact(Cursor::NW_Resize),
            "s-resize" => PropertyValue::Exact(Cursor::S_Resize),
            "se-resize" => PropertyValue::Exact(Cursor::SE_Resize),
            "sw-resize" => PropertyValue::Exact(Cursor::SW_Resize),
            "w-resize" => PropertyValue::Exact(Cursor::W_Resize),
            "we-resize" => PropertyValue::Exact(Cursor::WE_Resize),
            "ns-resize" => PropertyValue::Exact(Cursor::NS_Resize),
            "nesw-resize" => PropertyValue::Exact(Cursor::NESW_Resize),
            "nwse-resize" => PropertyValue::Exact(Cursor::NWSE_Resize),
            "col-resize" => PropertyValue::Exact(Cursor::ColResize),
            "row-resize" => PropertyValue::Exact(Cursor::RowResize),
            "all-scroll" => PropertyValue::Exact(Cursor::AllScroll),
            "zoom-in" => PropertyValue::Exact(Cursor::ZoomIn),
            "zoom-out" => PropertyValue::Exact(Cursor::ZoomOut),
            _ => return Err(parser.new_error_for_next_token()),
        })]),
        _ => return Err(parser.new_error_for_next_token()),
    }
}

fn parse_flex<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let mut result = Vec::new();

    while !parser.is_exhausted() {
        let token = parser.next()?;
        match token {
            Token::Number { value, .. } => match result.len() {
                0 => result.push(Property::FlexGrow(PropertyValue::Exact(*value))),
                1 => result.push(Property::FlexShrink(PropertyValue::Exact(*value))),
                2 => result.push(Property::FlexBasis(PropertyValue::Exact(Length::Px(*value)))),
                _ => return Err(parser.new_error_for_next_token()),
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
            }
            Token::Ident(s) => match_ignore_ascii_case! { s,
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
}

fn parse_flex_direction<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Ident(s) => Ok(vec![Property::FlexDirection(match_ignore_ascii_case! { s,
            "auto" => PropertyValue::Auto,
            "initial" => PropertyValue::Initial,
            "inherit" => PropertyValue::Inherit,
            "row" => PropertyValue::Exact(FlexDirection::Row),
            "row-reverse" => PropertyValue::Exact(FlexDirection::RowReverse),
            "column" => PropertyValue::Exact(FlexDirection::Column),
            "column-reverse" => PropertyValue::Exact(FlexDirection::ColumnReverse),
            _ => return Err(parser.new_error_for_next_token()),
        })]),
        _ => return Err(parser.new_error_for_next_token()),
    }
}

fn parse_flex_flow<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let mut result = Vec::new();

    while !parser.is_exhausted() {
        let token = parser.next()?;
        match token {
            Token::Ident(s) => match_ignore_ascii_case! { s,
                "initial" => {
                    result.push(Property::FlexDirection(PropertyValue::Initial));
                    result.push(Property::FlexWrap(PropertyValue::Initial));
                },
                "inherit" => {
                    result.push(Property::FlexDirection(PropertyValue::Inherit));
                    result.push(Property::FlexWrap(PropertyValue::Inherit));
                },
                // Flex Direction
                "row" => result.push(Property::FlexDirection(PropertyValue::Exact(FlexDirection::Row))),
                "row-reverse" => result.push(Property::FlexDirection(PropertyValue::Exact(FlexDirection::RowReverse))),
                "column" => result.push(Property::FlexDirection(PropertyValue::Exact(FlexDirection::Column))),
                "column-reverse" => result.push(Property::FlexDirection(PropertyValue::Exact(FlexDirection::ColumnReverse))),
                // Flex Wrap
                "no-wrap" => result.push(Property::FlexWrap(PropertyValue::Exact(FlexWrap::NoWrap))),
                "wrap" => result.push(Property::FlexWrap(PropertyValue::Exact(FlexWrap::Wrap))),
                "wrap-reverse" => result.push(Property::FlexWrap(PropertyValue::Exact(FlexWrap::WrapReverse))),
                _ => return Err(parser.new_error_for_next_token()),
            },
            _ => return Err(parser.new_error_for_next_token()),
        }
    }

    Ok(result)
}

fn parse_flex_wrap<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Ident(s) => Ok(vec![Property::FlexWrap(match_ignore_ascii_case! { s,
            "auto" => PropertyValue::Auto,
            "initial" => PropertyValue::Initial,
            "inherit" => PropertyValue::Inherit,
            "no-wrap" => PropertyValue::Exact(FlexWrap::NoWrap),
            "wrap" => PropertyValue::Exact(FlexWrap::Wrap),
            "wrap-reverse" => PropertyValue::Exact(FlexWrap::WrapReverse),
            _ => return Err(parser.new_error_for_next_token()),
        })]),
        _ => return Err(parser.new_error_for_next_token()),
    }
}

fn parse_font_family<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Ident(s) => Ok(vec![Property::FontFamily(PropertyValue::Exact(Arc::from(&**s)))]),
        _ => Err(parser.new_error_for_next_token()),
    }
}

fn parse_justify_content<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Ident(s) => Ok(vec![Property::JustifyContent(match_ignore_ascii_case! { s,
            "auto" => PropertyValue::Auto,
            "initial" => PropertyValue::Initial,
            "inherit" => PropertyValue::Inherit,
            "flex-start" => PropertyValue::Exact(JustifyContent::FlexStart),
            "flex-end" => PropertyValue::Exact(JustifyContent::FlexEnd),
            "center" => PropertyValue::Exact(JustifyContent::Center),
            "space-between" => PropertyValue::Exact(JustifyContent::SpaceBetween),
            "space-around" => PropertyValue::Exact(JustifyContent::SpaceAround),
            "space-evenly" => PropertyValue::Exact(JustifyContent::SpaceEvenly),
            _ => return Err(parser.new_error_for_next_token()),
        })]),
        _ => return Err(parser.new_error_for_next_token()),
    }
}

fn parse_margin<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let mut result = Vec::with_capacity(4);
    let sizes = parse_quad(parser)?;

    match sizes.len() {
        1 => {
            result.push(Property::MarginTop(sizes[0]));
            result.push(Property::MarginRight(sizes[0]));
            result.push(Property::MarginBottom(sizes[0]));
            result.push(Property::MarginLeft(sizes[0]));
        }
        2 => {
            result.push(Property::MarginTop(sizes[0]));
            result.push(Property::MarginRight(sizes[1]));
            result.push(Property::MarginBottom(sizes[0]));
            result.push(Property::MarginLeft(sizes[1]));
        }
        3 => {
            result.push(Property::MarginTop(sizes[0]));
            result.push(Property::MarginRight(sizes[1]));
            result.push(Property::MarginBottom(sizes[2]));
            result.push(Property::MarginLeft(sizes[1]));
        }
        4 => {
            result.push(Property::MarginTop(sizes[0]));
            result.push(Property::MarginRight(sizes[1]));
            result.push(Property::MarginBottom(sizes[2]));
            result.push(Property::MarginLeft(sizes[3]));
        }
        _ => return Err(parser.new_error_for_next_token()),
    }

    Ok(result)
}

fn parse_padding<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let mut result = Vec::with_capacity(4);
    let sizes = parse_quad(parser)?;

    match sizes.len() {
        1 => {
            result.push(Property::PaddingTop(sizes[0]));
            result.push(Property::PaddingRight(sizes[0]));
            result.push(Property::PaddingBottom(sizes[0]));
            result.push(Property::PaddingLeft(sizes[0]));
        }
        2 => {
            result.push(Property::PaddingTop(sizes[0]));
            result.push(Property::PaddingRight(sizes[1]));
            result.push(Property::PaddingBottom(sizes[0]));
            result.push(Property::PaddingLeft(sizes[1]));
        }
        3 => {
            result.push(Property::PaddingTop(sizes[0]));
            result.push(Property::PaddingRight(sizes[1]));
            result.push(Property::PaddingBottom(sizes[2]));
            result.push(Property::PaddingLeft(sizes[1]));
        }
        4 => {
            result.push(Property::PaddingTop(sizes[0]));
            result.push(Property::PaddingRight(sizes[1]));
            result.push(Property::PaddingBottom(sizes[2]));
            result.push(Property::PaddingLeft(sizes[3]));
        }
        _ => return Err(parser.new_error_for_next_token()),
    }

    Ok(result)
}

fn parse_position<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Vec<Property>, cssparser::ParseError<'i, ()>> {
    let token = parser.next()?;
    match token {
        Token::Ident(s) => Ok(vec![Property::Position(match_ignore_ascii_case! { s,
            "auto" => PropertyValue::Auto,
            "initial" => PropertyValue::Initial,
            "inherit" => PropertyValue::Inherit,
            "static" => PropertyValue::Exact(Position::Static),
            "relative" => PropertyValue::Exact(Position::Relative),
            "fixed" => PropertyValue::Exact(Position::Fixed),
            _ => return Err(parser.new_error_for_next_token()),
        })]),
        _ => return Err(parser.new_error_for_next_token()),
    }
}

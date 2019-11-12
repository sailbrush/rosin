#![allow(clippy::cognitive_complexity)]

use cssparser::*;

use crate::style::*;

const SELECTOR_LEN: usize = 5;

macro_rules! parse_single_dimension {
    ($property_type:expr, $parser:ident, $token:ident) => {
        match $token {
            Token::Number { .. } => {
                Ok(vec![$property_type(PropertyValue::Exact($token.into()))])
            }
            Token::Percentage { .. } => {
                Ok(vec![$property_type(PropertyValue::Exact($token.into()))])
            }
            Token::Dimension { .. } => {
                Ok(vec![$property_type(PropertyValue::Exact($token.into()))])
            }
            Token::Ident(s) => match s.to_lowercase().as_ref() {
                "auto" => Ok(vec![$property_type(PropertyValue::Auto)]),
                "none" => Ok(vec![$property_type(PropertyValue::None)]),
                "initial" => Ok(vec![$property_type(PropertyValue::Initial)]),
                "inherit" => Ok(vec![$property_type(PropertyValue::Inherit)]),
                _ => Err($parser.new_error_for_next_token()),
            },
            _ => Err($parser.new_error_for_next_token()), // TODO better error handling
        }
    };
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

    fn parse_prelude<'t>(
        &mut self,
        parser: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i, Self::Error>> {
        let mut specificity = 0;
        let mut selector_list: Vec<Selector> = Vec::with_capacity(SELECTOR_LEN);

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

                    selector_list.push(Selector::Class(s.replace("-", "_")));
                    specificity += 10;

                    whitespace = false;
                    direct = false;
                }
                Token::IDHash(s) | Token::Hash(s) => {
                    if !first && !direct && whitespace {
                        selector_list.push(Selector::Children);
                    }

                    selector_list.push(Selector::Id(s.replace("-", "_")));
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
        _location: SourceLocation,
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
        match name.to_lowercase().as_ref() {
            "color" => Ok(vec![Property::Color(PropertyValue::Exact(
                cssparser::Color::parse(parser)?,
            ))]),
            "font-size" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::FontSize, parser, token)
            }
            "width" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::Width, parser, token)
            }
            "max-width" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::MaxWidth, parser, token)
            }
            "min-width" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::MinWidth, parser, token)
            }
            "height" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::Height, parser, token)
            }
            "max-height" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::MaxHeight, parser, token)
            }
            "min-height" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::MinHeight, parser, token)
            }
            "top" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::Top, parser, token)
            }
            "right" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::Right, parser, token)
            }
            "bottom" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::Bottom, parser, token)
            }
            "left" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::Left, parser, token)
            }
            //"flex-direction" => {
            //    let token = parser.next()?;
            //    parse_single_dimension!(Property::FlexDirection, parser, token)
            //}
            "flex-grow" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::FlexGrow, parser, token)
            }
            "padding-top" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::PaddingTop, parser, token)
            }
            "padding-right" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::PaddingRight, parser, token)
            }
            "padding-bottom" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::PaddingBottom, parser, token)
            }
            "padding-left" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::PaddingLeft, parser, token)
            }
            "margin-top" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::MarginTop, parser, token)
            }
            "margin-right" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::MarginRight, parser, token)
            }
            "margin-bottom" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::MarginBottom, parser, token)
            }
            "margin-left" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::MarginLeft, parser, token)
            }
            "background-color" => Ok(vec![Property::BackgroundColor(PropertyValue::Exact(
                cssparser::Color::parse(parser)?,
            ))]),
            "border-top-color" => Ok(vec![Property::BorderTopColor(PropertyValue::Exact(
                cssparser::Color::parse(parser)?,
            ))]),
            "border-right-color" => Ok(vec![Property::BorderRightColor(PropertyValue::Exact(
                cssparser::Color::parse(parser)?,
            ))]),
            "border-bottom-color" => Ok(vec![Property::BorderBottomColor(PropertyValue::Exact(
                cssparser::Color::parse(parser)?,
            ))]),
            "border-left-color" => Ok(vec![Property::BorderLeftColor(PropertyValue::Exact(
                cssparser::Color::parse(parser)?,
            ))]),
            "border-top-width" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::BorderTopWidth, parser, token)
            }
            "border-right-width" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::BorderRightWidth, parser, token)
            }
            "border-bottom-width" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::BorderBottomWidth, parser, token)
            }
            "border-left-width" => {
                let token = parser.next()?;
                parse_single_dimension!(Property::BorderLeftWidth, parser, token)
            }
            _ => Err(parser.new_error_for_next_token()), // Unsupported property
        }
    }
}

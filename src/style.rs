#![allow(clippy::cognitive_complexity)]

use std::{cmp::Ordering, fs};

use cssparser::*;
use rayon::prelude::*;

use crate::dom::*;
use crate::parser::*;

#[derive(Debug, Clone, Copy)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Style {
    pub color: PropertyValue<cssparser::Color>,
    pub font_size: PropertyValue<Dimension>,

    pub width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_width: Option<f32>,

    pub height: Option<f32>,
    pub max_height: Option<f32>,
    pub min_height: Option<f32>,

    pub top: PropertyValue<Dimension>,
    pub right: PropertyValue<Dimension>,
    pub bottom: PropertyValue<Dimension>,
    pub left: PropertyValue<Dimension>,

    pub flex_direction: PropertyValue<FlexDirection>,
    pub flex_grow: f32,

    pub padding_top: PropertyValue<Dimension>,
    pub padding_right: PropertyValue<Dimension>,
    pub padding_bottom: PropertyValue<Dimension>,
    pub padding_left: PropertyValue<Dimension>,

    pub margin_top: PropertyValue<Dimension>,
    pub margin_right: PropertyValue<Dimension>,
    pub margin_bottom: PropertyValue<Dimension>,
    pub margin_left: PropertyValue<Dimension>,

    pub background_color: PropertyValue<cssparser::Color>,
    pub border_top_color: PropertyValue<cssparser::Color>,
    pub border_right_color: PropertyValue<cssparser::Color>,
    pub border_bottom_color: PropertyValue<cssparser::Color>,
    pub border_left_color: PropertyValue<cssparser::Color>,

    pub border_top_width: PropertyValue<Dimension>,
    pub border_right_width: PropertyValue<Dimension>,
    pub border_bottom_width: PropertyValue<Dimension>,
    pub border_left_width: PropertyValue<Dimension>,
}

#[derive(Debug)]
pub enum Selector {
    /// Represents a `*` selector
    Wildcard,

    /// Represents selectors beginning with `#`
    Id(String),

    /// Represents selectors beginning with `.`
    Class(String),

    /// Represents a ` ` selector relationship
    Children,

    /// Represents a `>` selector relationship
    DirectChildren,
}

impl Selector {
    /// Check if this selector applies to a node
    pub fn check<T>(&self, node: &Node<T>) -> bool {
        match self {
            Selector::Id(selector) => {
                if let Some(node_id) = &node.css_id {
                    *selector == *node_id
                } else {
                    false
                }
            }
            Selector::Class(selector) => {
                for class in &node.css_classes {
                    if *selector == *class {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Unit {
    None,
    Px,
    Em,
    Pt,
    Percent,
}

#[derive(Debug, Copy, Clone)]
pub enum Either {
    Float(f32),
    Int(i32),
}

#[derive(Debug, Copy, Clone)]
pub struct Dimension {
    pub value: Either,
    pub unit: Unit,
}

impl Default for Dimension {
    fn default() -> Self {
        Dimension {
            value: Either::Int(0),
            unit: Unit::None,
        }
    }
}

impl From<&Token<'_>> for Dimension {
    fn from(token: &Token) -> Self {
        match token {
            Token::Number {
                value, int_value, ..
            } => {
                if let Some(int) = int_value {
                    Dimension {
                        value: Either::Int(*int),
                        unit: Unit::None,
                    }
                } else {
                    Dimension {
                        value: Either::Float(*value),
                        unit: Unit::None,
                    }
                }
            }
            Token::Percentage { unit_value, .. } => Dimension {
                value: Either::Float(*unit_value),
                unit: Unit::Percent,
            },
            Token::Dimension {
                value,
                int_value,
                unit,
                ..
            } => {
                let new_unit = match unit.to_lowercase().as_ref() {
                    "px" => Unit::Px,
                    "em" => Unit::Em,
                    "pt" => Unit::Pt,
                    "%" => Unit::Percent,
                    _ => Unit::None,
                };

                if let Some(int) = int_value {
                    Dimension {
                        value: Either::Int(*int),
                        unit: new_unit,
                    }
                } else {
                    Dimension {
                        value: Either::Float(*value),
                        unit: new_unit,
                    }
                }
            }
            _ => panic!(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PropertyValue<T> {
    Auto,
    None,
    Initial,
    Inherit,
    Exact(T),
}

impl<T> Default for PropertyValue<T> {
    fn default() -> Self {
        PropertyValue::None
    }
}

#[derive(Debug)]
pub enum Property {
    Color(PropertyValue<cssparser::Color>),
    FontSize(PropertyValue<Dimension>),
    //FontFamily,
    //TextAlign,

    //LetterSpacing,
    //LineHeight,
    //WordSpacing,
    //TabWidth,
    //Cursor,

    //Display,
    //Float,
    //BoxSizing,
    Width(PropertyValue<Dimension>),
    MaxWidth(PropertyValue<Dimension>),
    MinWidth(PropertyValue<Dimension>),

    Height(PropertyValue<Dimension>),
    MaxHeight(PropertyValue<Dimension>),
    MinHeight(PropertyValue<Dimension>),

    //Position,
    Top(PropertyValue<Dimension>),
    Right(PropertyValue<Dimension>),
    Bottom(PropertyValue<Dimension>),
    Left(PropertyValue<Dimension>),

    //FlexWrap,
    FlexDirection(PropertyValue<FlexDirection>),
    FlexGrow(PropertyValue<Dimension>),
    //FlexShrink,
    //JustifyContent,
    //AlignItems,
    //AlignContent,

    //OverflowX,
    //OverflowY,
    PaddingTop(PropertyValue<Dimension>),
    PaddingRight(PropertyValue<Dimension>),
    PaddingBottom(PropertyValue<Dimension>),
    PaddingLeft(PropertyValue<Dimension>),

    MarginTop(PropertyValue<Dimension>),
    MarginRight(PropertyValue<Dimension>),
    MarginBottom(PropertyValue<Dimension>),
    MarginLeft(PropertyValue<Dimension>),

    //Background,
    //BackgroundImage,
    BackgroundColor(PropertyValue<cssparser::Color>),
    //BackgroundPosition,
    //BackgroundSize,
    //BackgroundRepeat,

    //BorderTopLeftRadius,
    //BorderTopRightRadius,
    //BorderBottomLeftRadius,
    //BorderBottomRightRadius,
    BorderTopColor(PropertyValue<cssparser::Color>),
    BorderRightColor(PropertyValue<cssparser::Color>),
    BorderBottomColor(PropertyValue<cssparser::Color>),
    BorderLeftColor(PropertyValue<cssparser::Color>),

    //BorderTopStyle,
    //BorderRightStyle,
    //BorderBottomStyle,
    //BorderLeftStyle,
    BorderTopWidth(PropertyValue<Dimension>),
    BorderRightWidth(PropertyValue<Dimension>),
    BorderBottomWidth(PropertyValue<Dimension>),
    BorderLeftWidth(PropertyValue<Dimension>),
    //BoxShadowTop,
    //BoxShadowRight,
    //BoxShadowBottom,
    //BoxShadowLeft,
}

#[derive(Debug)]
pub struct Rule {
    pub specificity: u32,
    pub selectors: Vec<Selector>,
    pub properties: Vec<Property>,
}

impl Eq for Rule {}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.specificity == other.specificity
    }
}

impl Ord for Rule {
    fn cmp(&self, other: &Self) -> Ordering {
        self.specificity.cmp(&other.specificity)
    }
}

impl PartialOrd for Rule {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[macro_export]
macro_rules! style_new {
    ($path:expr) => {
        if cfg!(debug_assertions) {
            Stylesheet::new_dynamic(concat!(env!("CARGO_MANIFEST_DIR"), $path))
        } else {
            Stylesheet::new_static(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $path)))
        }
    };
}

#[derive(Debug)]
pub struct Stylesheet {
    pub path: Option<&'static str>,
    pub rules: Vec<Rule>,
}

impl Default for Stylesheet {
    fn default() -> Self {
        Self {
            path: None,
            rules: Vec::with_capacity(0),
        }
    }
}

impl Stylesheet {
    pub fn new_static(text: &'static str) -> Self {
        Self {
            path: None,
            rules: Self::parse(text),
        }
    }

    pub fn new_dynamic(path: &'static str) -> Self {
        let mut new = Self {
            path: Some(path),
            rules: Vec::with_capacity(0),
        };
        new.reload();
        new
    }

    pub fn reload(&mut self) {
        let path = self.path.unwrap();
        let contents = fs::read_to_string(path).expect("[Rosin] Failed to read stylesheet.");
        self.rules = Self::parse(&contents);
    }

    /// Parse CSS text into rule list
    pub fn parse(text: &str) -> Vec<Rule> {
        let mut input = ParserInput::new(text);
        let mut parser = Parser::new(&mut input);
        let mut rules_list = Vec::new();

        for result in RuleListParser::new_for_stylesheet(&mut parser, RulesParser) {
            if let Ok(rule) = result {
                rules_list.push(rule);
            }
        }
        rules_list
    }

    /// Perform selector matching for a Dom tree
    pub fn style<T>(&self, dom: &Dom<T>) -> Vec<Style> {
        dom.arena
            .par_iter()
            .enumerate()
            .map(|(id, _node)| {
                // TODO use hashmap to store rules?
                let mut relevant_rules = self
                    .rules
                    .iter()
                    .filter(|rule| {
                        // Find matching rules
                        let mut direct = false;
                        let mut cmp_node = Some(id);
                        for (i, selector) in rule.selectors.iter().rev().enumerate() {
                            if let Some(n) = cmp_node {
                                loop {
                                    if i == 0 {
                                        if !selector.check(&dom.arena[n]) {
                                            return false;
                                        } else {
                                            cmp_node = dom.arena[n].parent;
                                            break; // Next selector
                                        }
                                    } else {
                                        match selector {
                                            Selector::Wildcard => {
                                                direct = false;
                                                cmp_node = dom.arena[n].parent;
                                                break; // Next selector
                                            }
                                            Selector::Id(_) | Selector::Class(_) => {
                                                if selector.check(&dom.arena[n]) {
                                                    direct = false;
                                                    cmp_node = dom.arena[n].parent;
                                                    break; // Next selector
                                                } else if direct {
                                                    return false;
                                                }
                                            }
                                            Selector::DirectChildren => {
                                                direct = true;
                                                break; // Next selector
                                            }
                                            Selector::Children => {
                                                direct = false;
                                                cmp_node = dom.arena[n].parent;
                                                break; // Next selector
                                            }
                                        }
                                    }
                                }
                            } else {
                                return false; // Made it to the leftmost selector unsasitfied
                            }
                        }
                        true
                    })
                    .collect::<Vec<&Rule>>();

                // Apply rules in order of specificity
                let mut computed_style = Style::default();
                relevant_rules.sort();
                relevant_rules.iter().for_each(|rule| {
                    for property in &rule.properties {
                        match property {
                            Property::Color(value) => {
                                computed_style.color = *value;
                            }
                            Property::FontSize(value) => {
                                computed_style.font_size = *value;
                            }

                            //FontFamily,
                            //TextAlign,

                            //LetterSpacing,
                            //LineHeight,
                            //WordSpacing,
                            //TabWidth,
                            //Cursor,

                            //Display,
                            //Float,
                            //BoxSizing,
                            Property::Width(value) => {
                                computed_style.width = match value {
                                    PropertyValue::Auto |
                                    PropertyValue::None |
                                    PropertyValue::Initial => { None },
                                    // TODO inherit properly
                                    PropertyValue::Inherit => { Some(0.0) },
                                    PropertyValue::Exact(flex_grow) => {
                                        match flex_grow.value {
                                            Either::Float(value) => { Some(value) }
                                            Either::Int(value) => {
                                                Some(value as f32)
                                            }
                                        }
                                    },
                                };
                            }
                            Property::MaxWidth(value) => {
                                computed_style.max_width = match value {
                                    PropertyValue::Auto |
                                    PropertyValue::None |
                                    PropertyValue::Initial => { None },
                                    // TODO inherit properly
                                    PropertyValue::Inherit => { Some(0.0) },
                                    PropertyValue::Exact(flex_grow) => {
                                        match flex_grow.value {
                                            Either::Float(value) => { Some(value) }
                                            Either::Int(value) => {
                                                Some(value as f32)
                                            }
                                        }
                                    },
                                };
                            }
                            Property::MinWidth(value) => {
                                computed_style.min_width = match value {
                                    PropertyValue::Auto |
                                    PropertyValue::None |
                                    PropertyValue::Initial => { None },
                                    // TODO inherit properly
                                    PropertyValue::Inherit => { Some(0.0) },
                                    PropertyValue::Exact(flex_grow) => {
                                        match flex_grow.value {
                                            Either::Float(value) => { Some(value) }
                                            Either::Int(value) => {
                                                Some(value as f32)
                                            }
                                        }
                                    },
                                };
                            }
                            Property::Height(value) => {
                                computed_style.height = match value {
                                    PropertyValue::Auto |
                                    PropertyValue::None |
                                    PropertyValue::Initial => { None },
                                    // TODO inherit properly
                                    PropertyValue::Inherit => { Some(0.0) },
                                    PropertyValue::Exact(flex_grow) => {
                                        match flex_grow.value {
                                            Either::Float(value) => { Some(value) }
                                            Either::Int(value) => {
                                                Some(value as f32)
                                            }
                                        }
                                    },
                                };
                            }
                            Property::MaxHeight(value) => {
                                computed_style.max_height  = match value {
                                    PropertyValue::Auto |
                                    PropertyValue::None |
                                    PropertyValue::Initial => { None },
                                    // TODO inherit properly
                                    PropertyValue::Inherit => { Some(0.0) },
                                    PropertyValue::Exact(flex_grow) => {
                                        match flex_grow.value {
                                            Either::Float(value) => { Some(value) }
                                            Either::Int(value) => {
                                                Some(value as f32)
                                            }
                                        }
                                    },
                                };
                            }
                            Property::MinHeight(value) => {
                                computed_style.min_height = match value {
                                    PropertyValue::Auto |
                                    PropertyValue::None |
                                    PropertyValue::Initial => { None },
                                    // TODO inherit properly
                                    PropertyValue::Inherit => { Some(0.0) },
                                    PropertyValue::Exact(flex_grow) => {
                                        match flex_grow.value {
                                            Either::Float(value) => { Some(value) }
                                            Either::Int(value) => {
                                                Some(value as f32)
                                            }
                                        }
                                    },
                                };
                            }

                            //Position,
                            Property::Top(value) => {
                                computed_style.top = *value;
                            }
                            Property::Right(value) => {
                                computed_style.right = *value;
                            }
                            Property::Bottom(value) => {
                                computed_style.bottom = *value;
                            }
                            Property::Left(value) => {
                                computed_style.left = *value;
                            }

                            //FlexWrap,
                            Property::FlexDirection(value) => {
                                computed_style.flex_direction = *value;
                            }
                            Property::FlexGrow(value) => {
                                computed_style.flex_grow = match value {
                                    PropertyValue::Auto |
                                    PropertyValue::None |
                                    PropertyValue::Initial => { 0.0 },
                                    // TODO inherit properly
                                    PropertyValue::Inherit => { 0.0 },
                                    PropertyValue::Exact(flex_grow) => {
                                        match flex_grow.value {
                                            Either::Float(value) => { value }
                                            Either::Int(value) => {
                                                value as f32
                                            }
                                        }
                                    },
                                }
                            }
                            //FlexShrink,
                            //JustifyContent,
                            //AlignItems,
                            //AlignContent,

                            //OverflowX,
                            //OverflowY,
                            Property::PaddingTop(value) => {
                                computed_style.padding_top = *value;
                            }
                            Property::PaddingRight(value) => {
                                computed_style.padding_right = *value;
                            }
                            Property::PaddingBottom(value) => {
                                computed_style.padding_bottom = *value;
                            }
                            Property::PaddingLeft(value) => {
                                computed_style.padding_left = *value;
                            }
                            Property::MarginTop(value) => {
                                computed_style.margin_top = *value;
                            }
                            Property::MarginRight(value) => {
                                computed_style.margin_right = *value;
                            }
                            Property::MarginBottom(value) => {
                                computed_style.margin_bottom = *value;
                            }
                            Property::MarginLeft(value) => {
                                computed_style.margin_left = *value;
                            }

                            //Background,
                            //BackgroundImage,
                            Property::BackgroundColor(value) => {
                                computed_style.background_color = *value;
                            }

                            //BackgroundPosition,
                            //BackgroundSize,
                            //BackgroundRepeat,

                            //BorderTopLeftRadius,
                            //BorderTopRightRadius,
                            //BorderBottomLeftRadius,
                            //BorderBottomRightRadius,
                            Property::BorderTopColor(value) => {
                                computed_style.border_top_color = *value;
                            }
                            Property::BorderRightColor(value) => {
                                computed_style.border_right_color = *value;
                            }
                            Property::BorderBottomColor(value) => {
                                computed_style.border_bottom_color = *value;
                            }
                            Property::BorderLeftColor(value) => {
                                computed_style.border_left_color = *value;
                            }

                            //BorderTopStyle,
                            //BorderRightStyle,
                            //BorderBottomStyle,
                            //BorderLeftStyle,
                            Property::BorderTopWidth(value) => {
                                computed_style.border_top_width = *value;
                            }
                            Property::BorderRightWidth(value) => {
                                computed_style.border_right_width = *value;
                            }
                            Property::BorderBottomWidth(value) => {
                                computed_style.border_bottom_width = *value;
                            }
                            Property::BorderLeftWidth(value) => {
                                computed_style.border_left_width = *value;
                            }
                        }
                    }
                });
                computed_style
            })
            .collect::<Vec<Style>>()
    }
}

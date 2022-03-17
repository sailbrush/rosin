#![forbid(unsafe_code)]

use crate::parser::*;
use crate::style::*;
use crate::tree::*;

use cssparser::{Parser, ParserInput, RuleListParser};

use std::sync::Arc;
use std::{cmp::Ordering, error::Error, fs, time::SystemTime};

macro_rules! apply {
    (@color, $value:expr, $style:expr, $par_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr.clone();
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$par_style {
                    $style.$attr = parent.$attr.clone();
                }
            }
            PropertyValue::Exact(color) => match color {
                cssparser::Color::CurrentColor => {
                    $style.$attr = $style.color.clone();
                }
                cssparser::Color::RGBA(rgba) => {
                    $style.$attr = *rgba;
                }
            },
            _ => debug_assert!(false),
        };
    };
    (@generic, $value:expr, $style:expr, $par_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$par_style {
                    $style.$attr = parent.$attr;
                }
            }
            PropertyValue::Exact(value) => {
                $style.$attr = *value;
            }
            _ => debug_assert!(false),
        };
    };
    (@generic_opt, $value:expr, $style:expr, $par_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$par_style {
                    $style.$attr = parent.$attr;
                }
            }
            PropertyValue::Exact(value) => {
                $style.$attr = Some(*value);
            }
            _ => debug_assert!(false),
        };
    };
    (@length, $value:expr, $style:expr, $par_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$par_style {
                    $style.$attr = parent.$attr;
                }
            }
            PropertyValue::Exact(value) => match value {
                Length::Em(value) => {
                    $style.$attr = $style.font_size * value;
                }
                Length::Px(value) => {
                    $style.$attr = *value;
                }
            },
            _ => debug_assert!(false),
        };
    };
    (@length_opt, $value:expr, $style:expr, $par_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Auto => $style.$attr = None,
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$par_style {
                    $style.$attr = parent.$attr;
                }
            }
            PropertyValue::Exact(value) => match value {
                Length::Em(value) => {
                    $style.$attr = Some($style.font_size * value);
                }
                Length::Px(value) => {
                    $style.$attr = Some(*value);
                }
            },
        };
    };
    (@length_max, $value:expr, $style:expr, $par_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Auto => $style.$attr = f32::INFINITY,
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$par_style {
                    $style.$attr = parent.$attr;
                }
            }
            PropertyValue::Exact(value) => match value {
                Length::Em(value) => {
                    $style.$attr = $style.font_size * value;
                }
                Length::Px(value) => {
                    $style.$attr = *value;
                }
            },
        };
    };
    (@length_min, $value:expr, $style:expr, $par_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Auto => $style.$attr = f32::NEG_INFINITY,
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$par_style {
                    $style.$attr = parent.$attr;
                }
            }
            PropertyValue::Exact(value) => match value {
                Length::Em(value) => {
                    $style.$attr = $style.font_size * value;
                }
                Length::Px(value) => {
                    $style.$attr = *value;
                }
            },
        };
    };
    (@str, $value:expr, $style:expr, $par_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Inherit => {
                if let Some(parent) = &$par_style {
                    $style.$attr = parent.$attr.clone();
                }
            }
            PropertyValue::Exact(value) => {
                $style.$attr = Some(value.clone());
            }
            _ => debug_assert!(false),
        };
    };
}

#[derive(Debug, Copy, Clone)]
pub enum Length {
    Px(f32),
    Em(f32),
}

#[derive(Debug, Copy, Clone)]
pub enum PropertyValue<T> {
    Auto,
    Initial,
    Inherit,
    Exact(T),
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
    FontFamily(PropertyValue<Arc<str>>),
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

#[derive(Debug, Clone)]
pub enum Selector {
    // Represents a `*` selector
    Wildcard,

    // Represents selectors beginning with `#`
    Id(String),

    // Represents selectors beginning with `.`
    Class(String),

    // Represents a ` ` selector relationship
    Children,

    // Represents a `>` selector relationship
    DirectChildren,
    // TODO - Represents a `:hover` selector
    //Hover,

    // TODO - Represents a `:focus` selector
    //Focus,
}

impl Selector {
    // Check if this selector applies to a node
    pub(crate) fn check<T>(&self, node: &ArrayNode<T>) -> bool {
        match self {
            Selector::Wildcard => true,
            Selector::Id(selector) | Selector::Class(selector) => node.classes.iter().any(|class| class == selector),
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Default, Clone)]
pub(crate) struct Stylesheet {
    pub path: Option<&'static str>,
    pub last_modified: Option<SystemTime>,
    pub rules: Vec<Rule>,
}

#[doc(hidden)]
impl Stylesheet {
    pub fn new_static(text: &'static str) -> Self {
        Self {
            path: None,
            last_modified: None,
            rules: Self::parse(text),
        }
    }

    pub fn new_dynamic(path: &'static str) -> Self {
        let mut new = Self {
            path: Some(path),
            last_modified: None,
            rules: Vec::new(),
        };
        new.poll().expect("[Rosin] Failed to load stylesheet.");
        new
    }

    // Parse CSS text into rule list
    pub fn parse(text: &str) -> Vec<Rule> {
        let mut input = ParserInput::new(text);
        let mut parser = Parser::new(&mut input);
        let mut rules_list = Vec::new();

        for rule in RuleListParser::new_for_stylesheet(&mut parser, RulesParser).flatten() {
            rules_list.push(rule);
        }
        rules_list
    }

    // Reload stylesheet if it changed on disk
    pub(crate) fn poll(&mut self) -> Result<bool, Box<dyn Error>> {
        if let Some(path) = self.path {
            let mut reload = true;
            let last_modified = fs::metadata(&path)?.modified()?;

            if let Some(prev_last_modified) = self.last_modified {
                if last_modified == prev_last_modified {
                    reload = false;
                }
            }

            if reload {
                self.last_modified = Some(last_modified);
                let contents = fs::read_to_string(path)?;
                self.rules = Self::parse(&contents);
            }

            Ok(reload)
        } else {
            Ok(false)
        }
    }

    // Perform selector matching and apply styles to a tree
    pub(crate) fn apply_style<T>(&self, tree: &mut [ArrayNode<T>]) {
        for id in 0..tree.len() {
            // TODO - benchmark hash map
            // TODO - use temp bump alloc instead of Vec
            let mut relevant_rules = self
                .rules
                .iter()
                .filter(|rule| {
                    // Find matching rules
                    // TODO - comment an explanation
                    let mut direct = false;
                    let mut cmp_node = Some(id);
                    for (i, selector) in rule.selectors.iter().rev().enumerate() {
                        loop {
                            if let Some(n) = cmp_node {
                                if i == 0 {
                                    if !selector.check(&tree[n]) {
                                        return false;
                                    } else {
                                        cmp_node = if n != 0 { Some(tree[n].parent) } else { None };
                                        break; // Next selector
                                    }
                                } else {
                                    match selector {
                                        Selector::Wildcard => {
                                            cmp_node = if n != 0 { Some(tree[n].parent) } else { None };
                                            direct = false;
                                            break; // Next selector
                                        }
                                        Selector::Id(_) | Selector::Class(_) => {
                                            cmp_node = if n != 0 { Some(tree[n].parent) } else { None };

                                            if selector.check(&tree[n]) {
                                                direct = false;
                                                break; // Next selector
                                            } else if direct {
                                                return false; // Must match, but didn't
                                            }

                                            direct = false;
                                            continue; // Don't go to the next selector, just move up the tree
                                        }
                                        Selector::DirectChildren => {
                                            direct = true;
                                            break; // Next selector
                                        }
                                        Selector::Children => {
                                            direct = false;
                                            break; // Next selector
                                        }
                                    }
                                }
                            } else {
                                return false; // Made it to the root unsasitfied
                            }
                        }
                    }
                    true // All selectors satisfied
                })
                .collect::<Vec<&Rule>>();

            let par_style: Option<Style> = if id == 0 { None } else { Some(tree[tree[id].parent].style.clone()) };

            relevant_rules.sort();

            // First find the font size and color (Used for relative lengths and currentColor)
            let mut font_size_set = false;
            let mut font_family_set = false;
            let mut color_set = false;
            relevant_rules.iter().for_each(|rule| {
                if font_size_set && font_family_set && color_set {
                    return;
                }
                for property in rule.properties.iter().rev() {
                    if font_size_set && font_family_set && color_set {
                        break;
                    }
                    match property {
                        Property::FontSize(value) => {
                            if font_size_set {
                                continue;
                            }
                            match value {
                                PropertyValue::Inherit => {
                                    if let Some(parent) = &par_style {
                                        tree[id].style.font_size = parent.font_size;
                                    }
                                }
                                PropertyValue::Exact(size) => match size {
                                    Length::Px(value) => {
                                        tree[id].style.font_size = *value;
                                    }
                                    Length::Em(value) => {
                                        if let Some(parent) = &par_style {
                                            tree[id].style.font_size = parent.font_size * value;
                                        } else {
                                            tree[id].style.font_size *= value;
                                        }
                                    }
                                },
                                _ => {}
                            };
                            font_size_set = true;
                        }
                        Property::FontFamily(value) => {
                            if font_family_set {
                                continue;
                            }
                            match value {
                                PropertyValue::Exact(family) => {
                                    tree[id].style.font_family = Some(family.clone());
                                }
                                _ => {
                                    // Inherited by default
                                    if let Some(parent) = &par_style {
                                        tree[id].style.font_family = parent.font_family.clone();
                                    }
                                }
                            }
                            font_family_set = true;
                        }
                        Property::Color(value) => {
                            if color_set {
                                continue;
                            }
                            match value {
                                PropertyValue::Initial => tree[id].style.color = Style::default().color,
                                PropertyValue::Exact(color) => {
                                    if let cssparser::Color::RGBA(rgba) = color {
                                        tree[id].style.color = *rgba;
                                    }
                                }
                                _ => {
                                    // Inherited by default
                                    if let Some(parent) = &par_style {
                                        tree[id].style.color = parent.color;
                                    }
                                }
                            }
                            color_set = true;
                        }
                        _ => {}
                    }
                }
            });
            if !font_size_set {
                if let Some(parent) = &par_style {
                    tree[id].style.font_size = parent.font_size;
                }
            }
            if !font_family_set {
                if let Some(parent) = &par_style {
                    tree[id].style.font_family = parent.font_family.clone();
                }
            }
            if !color_set {
                if let Some(parent) = &par_style {
                    tree[id].style.color = parent.color;
                }
            }

            relevant_rules.iter().for_each(|rule| {
                for property in &rule.properties {
                    match property {
                        Property::FontSize(_) => { /* already handled */ }
                        Property::Color(_) => { /* already handled */ }
                        Property::FontFamily(_) => { /* already handled */ }

                        Property::AlignContent(value) => {
                            apply!(@generic, value, tree[id].style, par_style, align_content);
                        }
                        Property::AlignItems(value) => {
                            apply!(@generic, value, tree[id].style, par_style, align_items);
                        }
                        Property::AlignSelf(value) => {
                            apply!(@generic, value, tree[id].style, par_style, align_self);
                        }
                        Property::BackgroundColor(value) => {
                            apply!(@color, value, tree[id].style, par_style, background_color);
                        }
                        // TODO - for gradients
                        /*Property::BackgroundImage(_) => {
                            todo!();
                            //apply!(@generic_opt, value, arena[id].style, par_style, background_image);
                        }*/
                        Property::BorderBottomColor(value) => {
                            apply!(@color, value, tree[id].style, par_style, border_bottom_color);
                        }
                        Property::BorderBottomLeftRadius(value) => {
                            apply!(@length, value, tree[id].style, par_style, border_bottom_left_radius);
                        }
                        Property::BorderBottomRightRadius(value) => {
                            apply!(@length, value, tree[id].style, par_style, border_bottom_right_radius);
                        }
                        Property::BorderBottomWidth(value) => {
                            apply!(@length, value, tree[id].style, par_style, border_bottom_width);
                        }
                        Property::BorderLeftColor(value) => {
                            apply!(@color, value, tree[id].style, par_style, border_left_color);
                        }
                        Property::BorderLeftWidth(value) => {
                            apply!(@length, value, tree[id].style, par_style, border_left_width);
                        }
                        Property::BorderRightColor(value) => {
                            apply!(@color, value, tree[id].style, par_style, border_right_color);
                        }
                        Property::BorderRightWidth(value) => {
                            apply!(@length, value, tree[id].style, par_style, border_right_width);
                        }
                        Property::BorderTopColor(value) => {
                            apply!(@color, value, tree[id].style, par_style, border_top_color);
                        }
                        Property::BorderTopLeftRadius(value) => {
                            apply!(@length, value, tree[id].style, par_style, border_top_left_radius);
                        }
                        Property::BorderTopRightRadius(value) => {
                            apply!(@length, value, tree[id].style, par_style, border_top_right_radius);
                        }
                        Property::BorderTopWidth(value) => {
                            apply!(@length, value, tree[id].style, par_style, border_top_width);
                        }
                        Property::Bottom(value) => {
                            apply!(@length_opt, value, tree[id].style, par_style, bottom);
                        }
                        Property::BoxShadowOffsetX(value) => {
                            apply!(@length, value, tree[id].style, par_style, box_shadow_offset_x);
                        }
                        Property::BoxShadowOffsetY(value) => {
                            apply!(@length, value, tree[id].style, par_style, box_shadow_offset_y);
                        }
                        Property::BoxShadowBlur(value) => {
                            apply!(@length, value, tree[id].style, par_style, box_shadow_blur);
                        }
                        Property::BoxShadowColor(value) => {
                            apply!(@color, value, tree[id].style, par_style, box_shadow_color);
                        }
                        Property::BoxShadowInset(value) => {
                            apply!(@generic_opt, value, tree[id].style, par_style, box_shadow_inset);
                        }
                        Property::Cursor(value) => {
                            apply!(@generic, value, tree[id].style, par_style, cursor);
                        }
                        Property::FlexBasis(value) => {
                            apply!(@length_opt, value, tree[id].style, par_style, flex_basis);
                        }
                        Property::FlexDirection(value) => {
                            apply!(@generic, value, tree[id].style, par_style, flex_direction);
                        }
                        Property::FlexGrow(value) => {
                            apply!(@generic, value, tree[id].style, par_style, flex_grow);
                        }
                        Property::FlexShrink(value) => {
                            apply!(@generic, value, tree[id].style, par_style, flex_shrink);
                        }
                        Property::FlexWrap(value) => {
                            apply!(@generic, value, tree[id].style, par_style, flex_wrap);
                        }
                        Property::FontWeight(value) => {
                            apply!(@generic, value, tree[id].style, par_style, font_weight);
                        }
                        Property::Height(value) => {
                            apply!(@length_opt, value, tree[id].style, par_style, height);
                        }
                        Property::JustifyContent(value) => {
                            apply!(@generic, value, tree[id].style, par_style, justify_content);
                        }
                        Property::Left(value) => {
                            apply!(@length_opt, value, tree[id].style, par_style, left);
                        }
                        Property::MarginBottom(value) => {
                            apply!(@length_opt, value, tree[id].style, par_style, margin_bottom);
                        }
                        Property::MarginLeft(value) => {
                            apply!(@length_opt, value, tree[id].style, par_style, margin_left);
                        }
                        Property::MarginRight(value) => {
                            apply!(@length_opt, value, tree[id].style, par_style, margin_right);
                        }
                        Property::MarginTop(value) => {
                            apply!(@length_opt, value, tree[id].style, par_style, margin_top);
                        }
                        Property::MaxHeight(value) => {
                            apply!(@length_max, value, tree[id].style, par_style, max_height);
                        }
                        Property::MaxWidth(value) => {
                            apply!(@length_max, value, tree[id].style, par_style, max_width);
                        }
                        Property::MinHeight(value) => {
                            apply!(@length_min, value, tree[id].style, par_style, min_height);
                        }
                        Property::MinWidth(value) => {
                            apply!(@length_min, value, tree[id].style, par_style, min_width);
                        }
                        Property::Opacity(value) => {
                            apply!(@generic, value, tree[id].style, par_style, opacity);
                        }
                        Property::Order(value) => {
                            apply!(@generic, value, tree[id].style, par_style, order);
                        }
                        Property::PaddingBottom(value) => {
                            apply!(@length, value, tree[id].style, par_style, padding_bottom);
                        }
                        Property::PaddingLeft(value) => {
                            apply!(@length, value, tree[id].style, par_style, padding_left);
                        }
                        Property::PaddingRight(value) => {
                            apply!(@length, value, tree[id].style, par_style, padding_right);
                        }
                        Property::PaddingTop(value) => {
                            apply!(@length, value, tree[id].style, par_style, padding_top);
                        }
                        Property::Position(value) => {
                            apply!(@generic, value, tree[id].style, par_style, position);
                        }
                        Property::Right(value) => {
                            apply!(@length_opt, value, tree[id].style, par_style, right);
                        }
                        Property::Top(value) => {
                            apply!(@length_opt, value, tree[id].style, par_style, top);
                        }
                        Property::Width(value) => {
                            apply!(@length_opt, value, tree[id].style, par_style, width);
                        }
                        Property::ZIndex(value) => {
                            apply!(@generic, value, tree[id].style, par_style, z_index);
                        }
                    }
                }
            });
        }
    }
}

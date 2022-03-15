#![forbid(unsafe_code)]
#![allow(clippy::cognitive_complexity)]

use crate::geometry::*;
use crate::parser::*;
use crate::tree::*;

use std::sync::Arc;
use std::{cmp::Ordering, error::Error, fs, time::SystemTime};

use cssparser::{Parser, ParserInput, RuleListParser, RGBA};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignContent {
    Center,
    FlexEnd,
    FlexStart,
    SpaceAround,
    SpaceBetween,
    Stretch,
}

impl AlignContent {
    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "stretch" => Ok(AlignContent::Stretch),
            "center" => Ok(AlignContent::Center),
            "flex-start" => Ok(AlignContent::FlexStart),
            "flex-end" => Ok(AlignContent::FlexEnd),
            "space-between" => Ok(AlignContent::SpaceBetween),
            "space-around" => Ok(AlignContent::SpaceAround),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignItems {
    Stretch,
    Center,
    FlexStart,
    FlexEnd,
}

impl AlignItems {
    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "stretch" => Ok(AlignItems::Stretch),
            "center" => Ok(AlignItems::Center),
            "flex-start" => Ok(AlignItems::FlexStart),
            "flex-end" => Ok(AlignItems::FlexEnd),
            _ => Err(()),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum Cursor {
    Default,
    None,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    VerticalText,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    E_Resize,
    N_Resize,
    NE_Resize,
    NW_Resize,
    S_Resize,
    SE_Resize,
    SW_Resize,
    W_Resize,
    WE_Resize,
    NS_Resize,
    NESW_Resize,
    NWSE_Resize,
    ColResize,
    RowResize,
    AllScroll,
    ZoomIn,
    ZoomOut,
}

impl Cursor {
    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "default" => Ok(Cursor::Default),
            "none" => Ok(Cursor::None),
            "context-menu" => Ok(Cursor::ContextMenu),
            "help" => Ok(Cursor::Help),
            "pointer" => Ok(Cursor::Pointer),
            "progress" => Ok(Cursor::Progress),
            "wait" => Ok(Cursor::Wait),
            "cell" => Ok(Cursor::Cell),
            "crosshair" => Ok(Cursor::Crosshair),
            "text" => Ok(Cursor::Text),
            "vertical-text" => Ok(Cursor::VerticalText),
            "alias" => Ok(Cursor::Alias),
            "copy" => Ok(Cursor::Copy),
            "move" => Ok(Cursor::Move),
            "no-drop" => Ok(Cursor::NoDrop),
            "not-allowed" => Ok(Cursor::NotAllowed),
            "grab" => Ok(Cursor::Grab),
            "grabbing" => Ok(Cursor::Grabbing),
            "e-resize" => Ok(Cursor::E_Resize),
            "n-resize" => Ok(Cursor::N_Resize),
            "ne-resize" => Ok(Cursor::NE_Resize),
            "nw-resize" => Ok(Cursor::NW_Resize),
            "s-resize" => Ok(Cursor::S_Resize),
            "se-resize" => Ok(Cursor::SE_Resize),
            "sw-resize" => Ok(Cursor::SW_Resize),
            "w-resize" => Ok(Cursor::W_Resize),
            "we-resize" => Ok(Cursor::WE_Resize),
            "ns-resize" => Ok(Cursor::NS_Resize),
            "nesw-resize" => Ok(Cursor::NESW_Resize),
            "nwse-resize" => Ok(Cursor::NWSE_Resize),
            "col-resize" => Ok(Cursor::ColResize),
            "row-resize" => Ok(Cursor::RowResize),
            "all-scroll" => Ok(Cursor::AllScroll),
            "zoom-in" => Ok(Cursor::ZoomIn),
            "zoom-out" => Ok(Cursor::ZoomOut),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl FlexDirection {
    pub fn is_row(&self) -> bool {
        match self {
            FlexDirection::Row | FlexDirection::RowReverse => true,
            FlexDirection::Column | FlexDirection::ColumnReverse => false,
        }
    }

    pub fn is_reverse(&self) -> bool {
        match self {
            FlexDirection::RowReverse | FlexDirection::ColumnReverse => true,
            FlexDirection::Row | FlexDirection::Column => false,
        }
    }

    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "row" => Ok(FlexDirection::Row),
            "row-reverse" => Ok(FlexDirection::RowReverse),
            "column" => Ok(FlexDirection::Column),
            "column-reverse" => Ok(FlexDirection::ColumnReverse),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

impl FlexWrap {
    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "no-wrap" => Ok(FlexWrap::NoWrap),
            "wrap" => Ok(FlexWrap::Wrap),
            "wrap-reverse" => Ok(FlexWrap::WrapReverse),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum JustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl JustifyContent {
    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "flex-start" => Ok(JustifyContent::FlexStart),
            "flex-end" => Ok(JustifyContent::FlexEnd),
            "center" => Ok(JustifyContent::Center),
            "space-between" => Ok(JustifyContent::SpaceBetween),
            "space-around" => Ok(JustifyContent::SpaceAround),
            "space-evenly" => Ok(JustifyContent::SpaceEvenly),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
    Static,
    Relative,
    Fixed,
}

impl Position {
    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "static" => Ok(Position::Static),
            "relative" => Ok(Position::Relative),
            "fixed" => Ok(Position::Fixed),
            _ => Err(()),
        }
    }
}

/// Computed style properties of a Node.
#[derive(Debug, Clone)]
pub struct Style {
    pub align_content: AlignContent,
    pub align_items: AlignItems,
    pub align_self: AlignItems,
    pub background_color: RGBA,
    //pub background_image: Option<piet::FixedGradient>,
    pub border_bottom_color: RGBA,
    pub border_bottom_left_radius: f32,
    pub border_bottom_right_radius: f32,
    pub border_bottom_width: f32,
    pub border_left_color: RGBA,
    pub border_left_width: f32,
    pub border_right_color: RGBA,
    pub border_right_width: f32,
    pub border_top_color: RGBA,
    pub border_top_left_radius: f32,
    pub border_top_right_radius: f32,
    pub border_top_width: f32,
    pub bottom: Option<f32>,
    pub box_shadow_offset_x: f32,
    pub box_shadow_offset_y: f32,
    pub box_shadow_blur: f32,
    pub box_shadow_color: RGBA,
    pub box_shadow_inset: Option<bool>,
    pub color: RGBA,
    pub cursor: Cursor,
    pub flex_basis: Option<f32>,
    pub flex_direction: FlexDirection,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_wrap: FlexWrap,
    pub font_family: Option<Arc<str>>,
    pub font_size: f32,
    pub font_weight: u32,
    pub height: Option<f32>,
    pub justify_content: JustifyContent,
    pub left: Option<f32>,
    pub margin_bottom: Option<f32>,
    pub margin_left: Option<f32>,
    pub margin_right: Option<f32>,
    pub margin_top: Option<f32>,
    pub max_height: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub min_width: f32,
    pub opacity: f32,
    pub order: i32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    pub padding_right: f32,
    pub padding_top: f32,
    pub position: Position,
    pub right: Option<f32>,
    pub top: Option<f32>,
    pub width: Option<f32>,
    pub z_index: i32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            align_content: AlignContent::Stretch,
            align_items: AlignItems::Stretch,
            align_self: AlignItems::Stretch,
            background_color: RGBA::transparent(),
            //background_image: None,
            border_bottom_color: RGBA::new(0, 0, 0, 255),
            border_bottom_left_radius: 0.0,
            border_bottom_right_radius: 0.0,
            border_bottom_width: 0.0,
            border_left_color: RGBA::new(0, 0, 0, 255),
            border_left_width: 0.0,
            border_right_color: RGBA::new(0, 0, 0, 255),
            border_right_width: 0.0,
            border_top_color: RGBA::new(0, 0, 0, 255),
            border_top_left_radius: 0.0,
            border_top_right_radius: 0.0,
            border_top_width: 0.0,
            bottom: None,
            box_shadow_offset_x: 0.0,
            box_shadow_offset_y: 0.0,
            box_shadow_blur: 0.0,
            box_shadow_color: RGBA::new(0, 0, 0, 255),
            box_shadow_inset: None,
            color: RGBA::new(0, 0, 0, 255),
            cursor: Cursor::Default,
            flex_basis: None,
            flex_direction: FlexDirection::Row,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_wrap: FlexWrap::NoWrap,
            font_family: None,
            font_size: 0.0,
            font_weight: 400,
            height: None,
            justify_content: JustifyContent::FlexStart,
            left: None,
            margin_bottom: Some(0.0),
            margin_left: Some(0.0),
            margin_right: Some(0.0),
            margin_top: Some(0.0),
            max_height: f32::INFINITY,
            max_width: f32::INFINITY,
            min_height: f32::NEG_INFINITY,
            min_width: f32::NEG_INFINITY,
            opacity: 1.0,
            order: 0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            padding_right: 0.0,
            padding_top: 0.0,
            position: Position::Static,
            right: None,
            top: None,
            width: None,
            z_index: 0,
        }
    }
}

impl Style {
    pub fn size(&self) -> Size {
        Size::new(self.width.unwrap_or(0.0), self.height.unwrap_or(0.0))
    }

    pub fn min_size(&self) -> Size {
        Size::new(self.min_width, self.min_height)
    }

    pub fn max_size(&self) -> Size {
        Size::new(self.max_width, self.max_height)
    }

    pub fn position(&self) -> Rect {
        Rect::new(
            self.top.unwrap_or(0.0),
            self.right.unwrap_or(0.0),
            self.bottom.unwrap_or(0.0),
            self.left.unwrap_or(0.0),
        )
    }

    pub fn margin(&self) -> Rect {
        Rect::new(
            self.margin_top.unwrap_or(0.0),
            self.margin_right.unwrap_or(0.0),
            self.margin_bottom.unwrap_or(0.0),
            self.margin_left.unwrap_or(0.0),
        )
    }

    pub fn border(&self) -> Rect {
        Rect::new(
            self.border_top_width,
            self.border_right_width,
            self.border_bottom_width,
            self.border_left_width,
        )
    }

    pub fn padding(&self) -> Rect {
        Rect::new(self.padding_top, self.padding_right, self.padding_bottom, self.padding_left)
    }
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
    // TODO - pass in a list of nodes that are hovered/focused
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

#![forbid(unsafe_code)]

use crate::geometry::*;

use cssparser::RGBA;

use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignContent {
    Center,
    FlexEnd,
    FlexStart,
    SpaceAround,
    SpaceBetween,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignItems {
    Stretch,
    Center,
    FlexStart,
    FlexEnd,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
    Static,
    Relative,
    Fixed,
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

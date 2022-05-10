#![forbid(unsafe_code)]

use crate::geometry::*;

use druid_shell::piet::{self, UnitPoint};

use std::{f32::consts::TAU, sync::Arc};

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

#[derive(Debug, Clone)]
pub enum GradientAngle {
    Top,
    Right,
    Bottom,
    Left,
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
    Degrees(f32),
}

#[derive(Debug, Clone)]
pub struct LinearGradient {
    pub angle: GradientAngle,
    pub gradient_stops: Vec<piet::GradientStop>,
}

impl LinearGradient {
    // Calculate the start and end points for a linear gradient
    pub fn resolve(&self, width: f32, height: f32) -> piet::LinearGradient {
        let start_point;
        let end_point;

        let calc = |mut rad: f32| {
            while rad < 0.0 {
                rad += TAU;
            }
            while rad >= TAU {
                rad -= TAU;
            }

            let u;
            let v;
            let hypot = width.hypot(height) / 2.0;
            if rad < TAU * 0.25 {
                let theta = (width / height).atan() - rad;
                let len = theta.cos() * hypot;
                let x = rad.sin() * len;
                let y = rad.cos() * len;
                u = (x / width) + 0.5;
                v = ((height / 2.0) - y) / height;
            } else if rad < TAU * 0.5 {
                let theta = rad - (TAU / 4.0) - (height / width).atan();
                let len = theta.cos() * hypot;
                let x = rad.sin() * len;
                let y = rad.cos() * len;
                u = (x / width) + 0.5;
                v = ((height / 2.0) - y) / height;
            } else if rad < TAU * 0.75 {
                let theta = (width / height).atan() - rad;
                let len = theta.cos() * hypot;
                let x = rad.sin() * len;
                let y = rad.cos() * len;
                u = 0.5 - (x / width);
                v = 1.0 - ((height / 2.0) - y) / height;
            } else {
                let theta = rad - (3.0 * TAU / 4.0) - (height / width).atan();
                let len = theta.cos() * hypot;
                let x = rad.sin() * len;
                let y = rad.cos() * len;
                u = 1.0 - ((width / 2.0) - x) / width;
                v = ((height / 2.0) - y) / height;
            }

            (UnitPoint::new(1.0 - u as f64, 1.0 - v as f64), UnitPoint::new(u as f64, v as f64))
        };

        match &self.angle {
            GradientAngle::Top => {
                start_point = UnitPoint::BOTTOM;
                end_point = UnitPoint::TOP;
            }
            GradientAngle::Right => {
                start_point = UnitPoint::LEFT;
                end_point = UnitPoint::RIGHT;
            }
            GradientAngle::Bottom => {
                start_point = UnitPoint::TOP;
                end_point = UnitPoint::BOTTOM;
            }
            GradientAngle::Left => {
                start_point = UnitPoint::RIGHT;
                end_point = UnitPoint::LEFT;
            }
            GradientAngle::TopRight => {
                (start_point, end_point) = calc((height / width).atan());
            }
            GradientAngle::TopLeft => {
                (start_point, end_point) = calc((width / height).atan() - TAU / 4.0);
            }
            GradientAngle::BottomRight => {
                (start_point, end_point) = calc((width / height).atan() + TAU / 4.0);
            }
            GradientAngle::BottomLeft => {
                (start_point, end_point) = calc((height / width).atan() + TAU / 2.0);
            }
            GradientAngle::Degrees(deg) => {
                (start_point, end_point) = calc(deg.to_radians());
            }
        }

        piet::LinearGradient::new(start_point, end_point, &*self.gradient_stops)
    }
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
    pub background_color: piet::Color,
    pub background_image: Option<Arc<Vec<LinearGradient>>>,
    pub border_bottom_color: piet::Color,
    pub border_bottom_left_radius: f32,
    pub border_bottom_right_radius: f32,
    pub border_bottom_width: f32,
    pub border_left_color: piet::Color,
    pub border_left_width: f32,
    pub border_right_color: piet::Color,
    pub border_right_width: f32,
    pub border_top_color: piet::Color,
    pub border_top_left_radius: f32,
    pub border_top_right_radius: f32,
    pub border_top_width: f32,
    pub bottom: Option<f32>,
    pub box_shadow_offset_x: f32,
    pub box_shadow_offset_y: f32,
    pub box_shadow_blur: f32,
    pub box_shadow_color: piet::Color,
    pub box_shadow_inset: Option<bool>,
    pub color: piet::Color,
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
            background_color: piet::Color::rgba8(0, 0, 0, 0),
            background_image: None,
            border_bottom_color: piet::Color::rgba8(0, 0, 0, 255),
            border_bottom_left_radius: 0.0,
            border_bottom_right_radius: 0.0,
            border_bottom_width: 0.0,
            border_left_color: piet::Color::rgba8(0, 0, 0, 255),
            border_left_width: 0.0,
            border_right_color: piet::Color::rgba8(0, 0, 0, 255),
            border_right_width: 0.0,
            border_top_color: piet::Color::rgba8(0, 0, 0, 255),
            border_top_left_radius: 0.0,
            border_top_right_radius: 0.0,
            border_top_width: 0.0,
            bottom: None,
            box_shadow_offset_x: 0.0,
            box_shadow_offset_y: 0.0,
            box_shadow_blur: 0.0,
            box_shadow_color: piet::Color::rgba8(0, 0, 0, 255),
            box_shadow_inset: None,
            color: piet::Color::rgba8(0, 0, 0, 255),
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

    pub fn trbl(&self) -> Rect {
        Rect::new(
            self.top.unwrap_or(f32::NAN),
            self.right.unwrap_or(f32::NAN),
            self.bottom.unwrap_or(f32::NAN),
            self.left.unwrap_or(f32::NAN),
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

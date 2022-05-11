#![forbid(unsafe_code)]

use std::sync::Arc;

use druid_shell::piet;

use crate::style::*;

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
    BackgroundImage(PropertyValue<Option<Arc<Vec<LinearGradient>>>>),
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
    BoxShadow(PropertyValue<Option<Arc<Vec<BoxShadow>>>>),
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

macro_rules! apply {
    (@color, $value:expr, $style:expr, $parent_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr.clone();
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$parent_style {
                    $style.$attr = parent.$attr.clone();
                }
            }
            PropertyValue::Exact(color) => match color {
                cssparser::Color::CurrentColor => {
                    $style.$attr = $style.color.clone();
                }
                cssparser::Color::RGBA(rgba) => {
                    $style.$attr = piet::Color::rgba8(rgba.red, rgba.green, rgba.blue, rgba.alpha);
                }
            },
            _ => debug_assert!(false),
        }
    };
    (@clone, $value:expr, $style:expr, $parent_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$parent_style {
                    if let Some(attribute) = &parent.$attr {
                        $style.$attr = Some(Arc::clone(attribute));
                    }
                } else {
                    $style.$attr = None;
                }
            }
            PropertyValue::Exact(value) => {
                $style.$attr = Some(Arc::clone(value));
            }
            _ => debug_assert!(false),
        }
    };
    (@clone_opt, $value:expr, $style:expr, $parent_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$parent_style {
                    if let Some(attribute) = &parent.$attr {
                        $style.$attr = Some(Arc::clone(attribute));
                    }
                } else {
                    $style.$attr = None;
                }
            }
            PropertyValue::Exact(None) => {
                $style.$attr = None;
            }
            PropertyValue::Exact(Some(value)) => {
                $style.$attr = Some(Arc::clone(value));
            }
            _ => debug_assert!(false),
        }
    };
    (@generic, $value:expr, $style:expr, $parent_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$parent_style {
                    $style.$attr = parent.$attr;
                }
            }
            PropertyValue::Exact(value) => {
                $style.$attr = *value;
            }
            _ => debug_assert!(false),
        }
    };
    (@generic_opt, $value:expr, $style:expr, $parent_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$parent_style {
                    $style.$attr = parent.$attr;
                }
            }
            PropertyValue::Exact(value) => {
                $style.$attr = Some(*value);
            }
            _ => debug_assert!(false),
        }
    };
    (@length, $value:expr, $style:expr, $parent_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$parent_style {
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
        }
    };
    (@length_opt, $value:expr, $style:expr, $parent_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Auto => $style.$attr = None,
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$parent_style {
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
        }
    };
    (@length_max, $value:expr, $style:expr, $parent_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Auto => $style.$attr = f32::INFINITY,
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$parent_style {
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
        }
    };
    (@length_min, $value:expr, $style:expr, $parent_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Auto => $style.$attr = f32::NEG_INFINITY,
            PropertyValue::Initial => {
                $style.$attr = Style::default().$attr;
            }
            PropertyValue::Inherit => {
                if let Some(parent) = &$parent_style {
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
        }
    };
    (@str, $value:expr, $style:expr, $parent_style:ident, $attr:ident) => {
        match $value {
            PropertyValue::Inherit => {
                if let Some(parent) = &$parent_style {
                    $style.$attr = parent.$attr.clone();
                }
            }
            PropertyValue::Exact(value) => {
                $style.$attr = Some(value.clone());
            }
            _ => debug_assert!(false),
        }
    };
}

impl Property {
    #[allow(clippy::assign_op_pattern)]
    pub fn apply(&self, style: &mut Style, parent_style: &Option<Style>) {
        match self {
            Property::AlignContent(value) => apply!(@generic, value, style, parent_style, align_content),
            Property::AlignItems(value) => apply!(@generic, value, style, parent_style, align_items),
            Property::AlignSelf(value) => apply!(@generic, value, style, parent_style, align_self),
            Property::BackgroundColor(value) => apply!(@color, value, style, parent_style, background_color),
            Property::BackgroundImage(value) => apply!(@clone_opt, value, style, parent_style, background_image),
            Property::BorderBottomColor(value) => apply!(@color, value, style, parent_style, border_bottom_color),
            Property::BorderBottomLeftRadius(value) => apply!(@length, value, style, parent_style, border_bottom_left_radius),
            Property::BorderBottomRightRadius(value) => apply!(@length, value, style, parent_style, border_bottom_right_radius),
            Property::BorderBottomWidth(value) => apply!(@length, value, style, parent_style, border_bottom_width),
            Property::BorderLeftColor(value) => apply!(@color, value, style, parent_style, border_left_color),
            Property::BorderLeftWidth(value) => apply!(@length, value, style, parent_style, border_left_width),
            Property::BorderRightColor(value) => apply!(@color, value, style, parent_style, border_right_color),
            Property::BorderRightWidth(value) => apply!(@length, value, style, parent_style, border_right_width),
            Property::BorderTopColor(value) => apply!(@color, value, style, parent_style, border_top_color),
            Property::BorderTopLeftRadius(value) => apply!(@length, value, style, parent_style, border_top_left_radius),
            Property::BorderTopRightRadius(value) => apply!(@length, value, style, parent_style, border_top_right_radius),
            Property::BorderTopWidth(value) => apply!(@length, value, style, parent_style, border_top_width),
            Property::Bottom(value) => apply!(@length_opt, value, style, parent_style, bottom),
            Property::BoxShadow(value) => apply!(@clone_opt, value, style, parent_style, box_shadow),
            Property::Cursor(value) => apply!(@generic, value, style, parent_style, cursor),
            Property::Color(value) => apply!(@color, value, style, parent_style, color),
            Property::FlexBasis(value) => apply!(@length_opt, value, style, parent_style, flex_basis),
            Property::FlexDirection(value) => apply!(@generic, value, style, parent_style, flex_direction),
            Property::FlexGrow(value) => apply!(@generic, value, style, parent_style, flex_grow),
            Property::FlexShrink(value) => apply!(@generic, value, style, parent_style, flex_shrink),
            Property::FlexWrap(value) => apply!(@generic, value, style, parent_style, flex_wrap),
            Property::FontFamily(value) => apply!(@clone, value, style, parent_style, font_family),
            Property::FontSize(value) => apply!(@length, value, style, parent_style, font_size),
            Property::FontWeight(value) => apply!(@generic, value, style, parent_style, font_weight),
            Property::Height(value) => apply!(@length_opt, value, style, parent_style, height),
            Property::JustifyContent(value) => apply!(@generic, value, style, parent_style, justify_content),
            Property::Left(value) => apply!(@length_opt, value, style, parent_style, left),
            Property::MarginBottom(value) => apply!(@length_opt, value, style, parent_style, margin_bottom),
            Property::MarginLeft(value) => apply!(@length_opt, value, style, parent_style, margin_left),
            Property::MarginRight(value) => apply!(@length_opt, value, style, parent_style, margin_right),
            Property::MarginTop(value) => apply!(@length_opt, value, style, parent_style, margin_top),
            Property::MaxHeight(value) => apply!(@length_max, value, style, parent_style, max_height),
            Property::MaxWidth(value) => apply!(@length_max, value, style, parent_style, max_width),
            Property::MinHeight(value) => apply!(@length_min, value, style, parent_style, min_height),
            Property::MinWidth(value) => apply!(@length_min, value, style, parent_style, min_width),
            Property::Opacity(value) => apply!(@generic, value, style, parent_style, opacity),
            Property::Order(value) => apply!(@generic, value, style, parent_style, order),
            Property::PaddingBottom(value) => apply!(@length, value, style, parent_style, padding_bottom),
            Property::PaddingLeft(value) => apply!(@length, value, style, parent_style, padding_left),
            Property::PaddingRight(value) => apply!(@length, value, style, parent_style, padding_right),
            Property::PaddingTop(value) => apply!(@length, value, style, parent_style, padding_top),
            Property::Position(value) => apply!(@generic, value, style, parent_style, position),
            Property::Right(value) => apply!(@length_opt, value, style, parent_style, right),
            Property::Top(value) => apply!(@length_opt, value, style, parent_style, top),
            Property::Width(value) => apply!(@length_opt, value, style, parent_style, width),
            Property::ZIndex(value) => apply!(@generic, value, style, parent_style, z_index),
        }
    }
}

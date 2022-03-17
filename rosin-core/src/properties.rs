#![forbid(unsafe_code)]

use std::sync::Arc;

use crate::style::*;

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

pub(crate) fn apply_properties(properties: &[Property], style: &mut Style, par_style: &Option<Style>) {
    for property in properties {
        match property {
            Property::FontSize(_) => { /* already handled */ }
            Property::Color(_) => { /* already handled */ }
            Property::FontFamily(_) => { /* already handled */ }

            Property::AlignContent(value) => {
                apply!(@generic, value, style, par_style, align_content);
            }
            Property::AlignItems(value) => {
                apply!(@generic, value, style, par_style, align_items);
            }
            Property::AlignSelf(value) => {
                apply!(@generic, value, style, par_style, align_self);
            }
            Property::BackgroundColor(value) => {
                apply!(@color, value, style, par_style, background_color);
            }
            // TODO - for gradients
            /*Property::BackgroundImage(_) => {
                todo!();
                //apply!(@generic_opt, value, arena[id].style, par_style, background_image);
            }*/
            Property::BorderBottomColor(value) => {
                apply!(@color, value, style, par_style, border_bottom_color);
            }
            Property::BorderBottomLeftRadius(value) => {
                apply!(@length, value, style, par_style, border_bottom_left_radius);
            }
            Property::BorderBottomRightRadius(value) => {
                apply!(@length, value, style, par_style, border_bottom_right_radius);
            }
            Property::BorderBottomWidth(value) => {
                apply!(@length, value, style, par_style, border_bottom_width);
            }
            Property::BorderLeftColor(value) => {
                apply!(@color, value, style, par_style, border_left_color);
            }
            Property::BorderLeftWidth(value) => {
                apply!(@length, value, style, par_style, border_left_width);
            }
            Property::BorderRightColor(value) => {
                apply!(@color, value, style, par_style, border_right_color);
            }
            Property::BorderRightWidth(value) => {
                apply!(@length, value, style, par_style, border_right_width);
            }
            Property::BorderTopColor(value) => {
                apply!(@color, value, style, par_style, border_top_color);
            }
            Property::BorderTopLeftRadius(value) => {
                apply!(@length, value, style, par_style, border_top_left_radius);
            }
            Property::BorderTopRightRadius(value) => {
                apply!(@length, value, style, par_style, border_top_right_radius);
            }
            Property::BorderTopWidth(value) => {
                apply!(@length, value, style, par_style, border_top_width);
            }
            Property::Bottom(value) => {
                apply!(@length_opt, value, style, par_style, bottom);
            }
            Property::BoxShadowOffsetX(value) => {
                apply!(@length, value, style, par_style, box_shadow_offset_x);
            }
            Property::BoxShadowOffsetY(value) => {
                apply!(@length, value, style, par_style, box_shadow_offset_y);
            }
            Property::BoxShadowBlur(value) => {
                apply!(@length, value, style, par_style, box_shadow_blur);
            }
            Property::BoxShadowColor(value) => {
                apply!(@color, value, style, par_style, box_shadow_color);
            }
            Property::BoxShadowInset(value) => {
                apply!(@generic_opt, value, style, par_style, box_shadow_inset);
            }
            Property::Cursor(value) => {
                apply!(@generic, value, style, par_style, cursor);
            }
            Property::FlexBasis(value) => {
                apply!(@length_opt, value, style, par_style, flex_basis);
            }
            Property::FlexDirection(value) => {
                apply!(@generic, value, style, par_style, flex_direction);
            }
            Property::FlexGrow(value) => {
                apply!(@generic, value, style, par_style, flex_grow);
            }
            Property::FlexShrink(value) => {
                apply!(@generic, value, style, par_style, flex_shrink);
            }
            Property::FlexWrap(value) => {
                apply!(@generic, value, style, par_style, flex_wrap);
            }
            Property::FontWeight(value) => {
                apply!(@generic, value, style, par_style, font_weight);
            }
            Property::Height(value) => {
                apply!(@length_opt, value, style, par_style, height);
            }
            Property::JustifyContent(value) => {
                apply!(@generic, value, style, par_style, justify_content);
            }
            Property::Left(value) => {
                apply!(@length_opt, value, style, par_style, left);
            }
            Property::MarginBottom(value) => {
                apply!(@length_opt, value, style, par_style, margin_bottom);
            }
            Property::MarginLeft(value) => {
                apply!(@length_opt, value, style, par_style, margin_left);
            }
            Property::MarginRight(value) => {
                apply!(@length_opt, value, style, par_style, margin_right);
            }
            Property::MarginTop(value) => {
                apply!(@length_opt, value, style, par_style, margin_top);
            }
            Property::MaxHeight(value) => {
                apply!(@length_max, value, style, par_style, max_height);
            }
            Property::MaxWidth(value) => {
                apply!(@length_max, value, style, par_style, max_width);
            }
            Property::MinHeight(value) => {
                apply!(@length_min, value, style, par_style, min_height);
            }
            Property::MinWidth(value) => {
                apply!(@length_min, value, style, par_style, min_width);
            }
            Property::Opacity(value) => {
                apply!(@generic, value, style, par_style, opacity);
            }
            Property::Order(value) => {
                apply!(@generic, value, style, par_style, order);
            }
            Property::PaddingBottom(value) => {
                apply!(@length, value, style, par_style, padding_bottom);
            }
            Property::PaddingLeft(value) => {
                apply!(@length, value, style, par_style, padding_left);
            }
            Property::PaddingRight(value) => {
                apply!(@length, value, style, par_style, padding_right);
            }
            Property::PaddingTop(value) => {
                apply!(@length, value, style, par_style, padding_top);
            }
            Property::Position(value) => {
                apply!(@generic, value, style, par_style, position);
            }
            Property::Right(value) => {
                apply!(@length_opt, value, style, par_style, right);
            }
            Property::Top(value) => {
                apply!(@length_opt, value, style, par_style, top);
            }
            Property::Width(value) => {
                apply!(@length_opt, value, style, par_style, width);
            }
            Property::ZIndex(value) => {
                apply!(@generic, value, style, par_style, z_index);
            }
        }
    }
}

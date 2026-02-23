use std::{f64::consts::TAU, fmt::Display, sync::Arc};

use kurbo::{Affine, Point, Rect};
use parley::Alignment;
use vello::peniko::{
    self,
    color::{ColorSpaceTag, HueDirection},
};

use crate::css::properties::ColorProperty;

/// A `text-align` CSS value.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Start,
    End,
    Left,
    Right,
    Center,
    Justify,
}

impl std::fmt::Display for TextAlign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextAlign::Start => f.write_str("start"),
            TextAlign::End => f.write_str("end"),
            TextAlign::Left => f.write_str("left"),
            TextAlign::Right => f.write_str("right"),
            TextAlign::Center => f.write_str("center"),
            TextAlign::Justify => f.write_str("justify"),
        }
    }
}

impl From<TextAlign> for Alignment {
    fn from(val: TextAlign) -> Self {
        match val {
            TextAlign::Start => Alignment::Start,
            TextAlign::End => Alignment::End,
            TextAlign::Left => Alignment::Left,
            TextAlign::Right => Alignment::Right,
            TextAlign::Center => Alignment::Center,
            TextAlign::Justify => Alignment::Justify,
        }
    }
}

// ---------- Length ----------

/// A definite length value.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Length {
    Px(f32),
    Em(f32),
}

impl Eq for Length {}

impl Default for Length {
    fn default() -> Self {
        Length::Px(0.0)
    }
}

impl Length {
    pub const ZERO: Length = Length::Px(0.0);

    /// Resolves the length to a px value using the provided font size.
    #[inline]
    pub fn resolve(&self, font_size: f32) -> f32 {
        match *self {
            Length::Px(px) => px,
            Length::Em(em) => em * font_size,
        }
    }
}

impl Display for Length {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Length::Px(v) => write!(f, "{v}px"),
            Length::Em(v) => write!(f, "{v}em"),
        }
    }
}

impl From<f32> for Length {
    fn from(value: f32) -> Self {
        Self::Px(value)
    }
}

impl From<f64> for Length {
    fn from(value: f64) -> Self {
        Self::Px(value as f32)
    }
}

// ---------- Box Shadow ----------

/// A `box-shadow` CSS value.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct BoxShadow {
    /// Horizontal offset of the shadow.
    pub offset_x: Length,
    /// Vertical offset of the shadow.
    pub offset_y: Length,
    /// Blur radius of the shadow.
    pub blur: Length,
    /// Spread radius of the shadow (positive grows, negative shrinks).
    pub spread: Length,
    /// Shadow color. `None` is equivalent to `currentcolor`.
    pub color: Option<peniko::Color>,
    /// Inset shadows are currently unsupported.
    pub inset: bool, // TODO - update doc comment when they are.
}

impl Eq for BoxShadow {}

impl Display for BoxShadow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.inset {
            f.write_str("inset ")?;
        }

        write!(f, "{} {} {} {}", self.offset_x, self.offset_y, self.blur, self.spread)?;

        if let Some(color) = self.color {
            let color = color.to_rgba8();
            write!(f, " #{:02X}{:02X}{:02X}{:02X}", color.r, color.g, color.b, color.a)?;
        } else {
            write!(f, " currentcolor")?;
        }

        Ok(())
    }
}

// ---------- Text Shadow ----------

/// A `text-shadow` CSS value.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct TextShadow {
    /// Horizontal offset of the shadow.
    pub offset_x: Length,
    /// Vertical offset of the shadow.
    pub offset_y: Length,
    /// Blur radius of the shadow.
    pub blur: Length,
    /// Shadow color. `None` is equivalent to `currentcolor`.
    pub color: Option<peniko::Color>,
}

impl Eq for TextShadow {}

impl Display for TextShadow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.offset_x, self.offset_y, self.blur)?;

        if let Some(color) = self.color {
            let color = color.to_rgba8();
            write!(f, " #{:02X}{:02X}{:02X}{:02X}", color.r, color.g, color.b, color.a)?;
        } else {
            write!(f, " currentcolor")?;
        }

        Ok(())
    }
}

// ---------- Direction ----------

/// A CSS value that determines the direction that a node's children should be laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Row => f.write_str("row"),
            Direction::RowReverse => f.write_str("row-reverse"),
            Direction::Column => f.write_str("column"),
            Direction::ColumnReverse => f.write_str("column-reverse"),
        }
    }
}

impl Direction {
    pub fn is_row(&self) -> bool {
        match self {
            Direction::Row | Direction::RowReverse => true,
            Direction::Column | Direction::ColumnReverse => false,
        }
    }

    pub fn is_reverse(&self) -> bool {
        match self {
            Direction::RowReverse | Direction::ColumnReverse => true,
            Direction::Column | Direction::Row => false,
        }
    }

    pub fn other_axis(&self) -> Self {
        match self {
            Direction::Row => Direction::Column,
            Direction::RowReverse => Direction::ColumnReverse,
            Direction::Column => Direction::Row,
            Direction::ColumnReverse => Direction::RowReverse,
        }
    }
}

// ---------- Gradient ----------

/// An angle/direction for `linear-gradient(...)`.
#[derive(Debug, Copy, Clone)]
pub enum GradientAngle {
    ToTop,
    ToRight,
    ToBottom,
    ToLeft,
    ToTopRight,
    ToTopLeft,
    ToBottomRight,
    ToBottomLeft,
    Radians(f32),
    Degrees(f32),
}

impl PartialEq for GradientAngle {
    fn eq(&self, other: &Self) -> bool {
        use GradientAngle::*;

        const EPS: f32 = 1.0e-6;

        fn radians(a: &GradientAngle) -> Option<f32> {
            match a {
                Radians(r) => Some(*r),
                Degrees(d) => Some(d.to_radians()),
                _ => None,
            }
        }

        fn approx_eq(a: f32, b: f32) -> bool {
            // avoid treating NaN as equal to anything
            if a.is_nan() || b.is_nan() {
                return false;
            }
            (a - b).abs() <= EPS
        }

        match (self, other) {
            // keyword directions only equal to themselves
            (ToTop, ToTop)
            | (ToRight, ToRight)
            | (ToBottom, ToBottom)
            | (ToLeft, ToLeft)
            | (ToTopRight, ToTopRight)
            | (ToTopLeft, ToTopLeft)
            | (ToBottomRight, ToBottomRight)
            | (ToBottomLeft, ToBottomLeft) => true,

            // compare angles in radians
            _ => match (radians(self), radians(other)) {
                (Some(a), Some(b)) => approx_eq(a, b),
                _ => false,
            },
        }
    }
}

impl Display for GradientAngle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GradientAngle::ToTop => f.write_str("to top"),
            GradientAngle::ToRight => f.write_str("to right"),
            GradientAngle::ToBottom => f.write_str("to bottom"),
            GradientAngle::ToLeft => f.write_str("to left"),
            GradientAngle::ToTopRight => f.write_str("to top right"),
            GradientAngle::ToTopLeft => f.write_str("to top left"),
            GradientAngle::ToBottomRight => f.write_str("to bottom right"),
            GradientAngle::ToBottomLeft => f.write_str("to bottom left"),
            GradientAngle::Radians(value) => write!(f, "{value}rad"),
            GradientAngle::Degrees(value) => write!(f, "{value}deg"),
        }
    }
}

/// A `linear-gradient(...)` CSS value.
#[derive(Debug, Clone)]
pub struct LinearGradient {
    pub(crate) angle: GradientAngle,
    pub(crate) gradient_stops: Vec<(f32, ColorProperty)>,
    pub(crate) interpolation_cs: ColorSpaceTag,
    pub(crate) hue_direction: HueDirection,
}

impl PartialEq for LinearGradient {
    fn eq(&self, other: &Self) -> bool {
        const EPS: f32 = 1.0e-6;

        fn approx_eq(a: f32, b: f32) -> bool {
            if a.is_nan() || b.is_nan() {
                return false;
            }
            (a - b).abs() <= EPS
        }

        if self.angle != other.angle {
            return false;
        }
        if self.interpolation_cs != other.interpolation_cs {
            return false;
        }
        if self.hue_direction != other.hue_direction {
            return false;
        }

        if self.gradient_stops.len() != other.gradient_stops.len() {
            return false;
        }

        self.gradient_stops
            .iter()
            .zip(other.gradient_stops.iter())
            .all(|((t1, c1), (t2, c2))| approx_eq(*t1, *t2) && c1 == c2)
    }
}

impl Display for LinearGradient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use cssparser::ToCss;

        f.write_str("linear-gradient(")?;
        self.angle.fmt(f)?;
        f.write_str(", ")?;
        for (i, (offset, property_color)) in self.gradient_stops.iter().enumerate() {
            match property_color {
                ColorProperty::CurrentColor => cssparser_color::Color::CurrentColor,
                ColorProperty::Color(color) => {
                    let color = color.to_rgba8();
                    cssparser_color::Color::Rgba(cssparser_color::RgbaLegacy {
                        red: color.r,
                        green: color.g,
                        blue: color.b,
                        alpha: color.a as f32 / 255.0,
                    })
                }
            }
            .to_css(f)?;
            let percent = (offset * 100.0) as u32;
            if percent != 0 && percent != 100 {
                write!(f, " {percent}%")?;
            }
            if i != self.gradient_stops.len() - 1 {
                f.write_str(", ")?;
            }
        }
        f.write_str(")")
    }
}

impl LinearGradient {
    /// Creates a new `LinearGradient` with the given direction/angle and no stops.
    ///
    /// You must add stops with [`LinearGradient::add_stop`] before the gradient is usable.
    /// A valid gradient needs at least two stops.
    pub fn new(angle: impl Into<Option<GradientAngle>>) -> Self {
        Self {
            angle: angle.into().unwrap_or(GradientAngle::ToBottom),
            gradient_stops: Vec::new(),
            interpolation_cs: ColorSpaceTag::Srgb,
            hue_direction: HueDirection::default(),
        }
    }

    /// Add a stop to the gradient. It must have at least two stops to be valid. `color: None` is treated as `currentColor`
    pub fn add_stop(mut self, offset: f32, color: impl Into<Option<peniko::Color>>) -> Self {
        if let Some(color) = color.into() {
            self.gradient_stops.push((offset.clamp(0.0, 1.0), ColorProperty::Color(color)));
        } else {
            self.gradient_stops.push((offset.clamp(0.0, 1.0), ColorProperty::CurrentColor));
        }
        self
    }

    /// Sets the interpolation color space used when blending between stops.
    pub fn with_interpolation_space(mut self, cs: ColorSpaceTag) -> Self {
        self.interpolation_cs = cs;
        self
    }

    /// Sets the hue interpolation direction used by hue-based color spaces.
    pub fn with_hue_direction(mut self, dir: HueDirection) -> Self {
        self.hue_direction = dir;
        self
    }

    /// Calculate the start and end points for a gradient, and resolve currentColor.
    pub fn resolve(&self, rect: Rect, current_color: peniko::Color) -> peniko::Gradient {
        let width = rect.width();
        let height = rect.height();

        let mut start;
        let mut end;

        let calc = |mut rad: f64| {
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

            (Point::new(1.0 - u, 1.0 - v), Point::new(u, v))
        };

        match &self.angle {
            GradientAngle::ToTop => {
                start = Point::new(0.5, 1.0);
                end = Point::new(0.5, 0.0);
            }
            GradientAngle::ToRight => {
                start = Point::new(0.0, 0.5);
                end = Point::new(1.0, 0.5);
            }
            GradientAngle::ToBottom => {
                start = Point::new(0.5, 0.0);
                end = Point::new(0.5, 1.0);
            }
            GradientAngle::ToLeft => {
                start = Point::new(1.0, 0.5);
                end = Point::new(0.0, 0.5);
            }
            GradientAngle::ToTopRight => {
                (start, end) = calc((height / width).atan());
            }
            GradientAngle::ToTopLeft => {
                (start, end) = calc((width / height).atan() - TAU / 4.0);
            }
            GradientAngle::ToBottomRight => {
                (start, end) = calc((width / height).atan() + TAU / 4.0);
            }
            GradientAngle::ToBottomLeft => {
                (start, end) = calc((height / width).atan() + TAU / 2.0);
            }
            GradientAngle::Degrees(deg) => {
                (start, end) = calc(deg.to_radians() as f64);
            }
            GradientAngle::Radians(rad) => {
                (start, end) = calc(*rad as f64);
            }
        }

        start.x = start.x * width + rect.origin().x;
        start.y = start.y * height + rect.origin().y;

        end.x = end.x * width + rect.origin().x;
        end.y = end.y * height + rect.origin().y;

        let mut stops = peniko::ColorStops::new();
        for &(offset, color) in &self.gradient_stops {
            stops.push(peniko::ColorStop {
                offset,
                color: color.resolve(current_color).into(),
            });
        }

        peniko::Gradient {
            kind: peniko::GradientKind::Linear(peniko::LinearGradientPosition { start, end }),
            extend: peniko::Extend::default(),
            stops,
            interpolation_cs: self.interpolation_cs,
            ..Default::default()
        }
    }
}

/// A stack of gradients used in the `background-image` property.
///
/// Can be created with a [`GradientStackBuilder`]
#[derive(Clone, Debug, PartialEq)]
pub struct GradientStack {
    pub(crate) stack: Arc<Vec<LinearGradient>>,
}

impl std::fmt::Display for GradientStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, gradient) in self.stack.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{gradient}")?;
        }
        Ok(())
    }
}

/// Used to build a [`GradientStack`].
#[derive(Default)]
pub struct GradientStackBuilder {
    stack: Vec<LinearGradient>,
}

impl GradientStackBuilder {
    /// Creates an empty gradient stack.
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Pushes a `LinearGradient` onto the stack.
    pub fn add_linear(mut self, gradient: LinearGradient) -> Self {
        self.stack.push(gradient);
        self
    }

    /// Finalizes the gradient stack.
    pub fn build(self) -> GradientStack {
        GradientStack { stack: Arc::new(self.stack) }
    }
}

// ---------- Unit ----------

/// A potentially flexible length value.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Unit {
    Auto,
    Em(f32),
    Percent(f32),
    Px(f32),
    Stretch(f32),
}

impl Eq for Unit {}

impl Default for Unit {
    fn default() -> Self {
        Self::Px(0.0)
    }
}

impl Unit {
    /// Returns `true` if the value is a definite length.
    pub fn is_definite(&self) -> bool {
        matches!(self, Unit::Em(_) | Unit::Percent(_) | Unit::Px(_))
    }

    /// Computes the length if it's definite, otherwise returns `0.0`.
    pub fn definite_size(&self, font_size: f32, pct_base: f32) -> f32 {
        match self {
            Unit::Em(em) => em * font_size,
            Unit::Percent(pct) => pct * pct_base,
            Unit::Px(px) => *px,
            _ => 0.0,
        }
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unit::Auto => write!(f, "auto"),
            Unit::Em(em) => write!(f, "{em}em"),
            Unit::Percent(value) => write!(f, "{}%", value * 100.0),
            Unit::Px(value) => write!(f, "{value}px"),
            Unit::Stretch(value) => write!(f, "{value}s"),
        }
    }
}

// ---------- Position ----------

/// A CSS value that determines how a node should be laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    ParentDirected,
    SelfDirected,
    Fixed,
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Position::ParentDirected => f.write_str("parent-directed"),
            Position::SelfDirected => f.write_str("self-directed"),
            Position::Fixed => f.write_str("fixed"),
        }
    }
}

// ---------- Font Style ----------

/// All of the properties needed for text layout.
///
/// Returned by [`Style::get_font_layout_style`].
///
/// Intended to be used for text layout cache invalidation.
/// If these changed, the cache is invalid.
#[derive(Clone, Debug, PartialEq)]
pub struct FontLayoutStyle {
    pub font_family: Option<Arc<str>>,
    pub font_size: f32,
    pub font_style: parley::style::FontStyle,
    pub font_weight: f32,
    pub font_width: f32,
    pub line_height: Unit,
    pub letter_spacing: Option<Unit>,
    pub word_spacing: Option<Unit>,
    pub text_align: TextAlign,
}

// ---------- Layout Style ----------

/// All of the properties that can affect layout.
/// Returned by [`Style::get_layout_style`]
///
/// Intended to be used for layout cache invalidation.
/// If these changed, layout should be considered invalid.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayoutStyle {
    // This need to match the values in Property::affects_layout
    pub border_bottom_left_radius: Length,
    pub border_bottom_right_radius: Length,
    pub border_bottom_width: Length,
    pub border_left_width: Length,
    pub border_right_width: Length,
    pub border_top_left_radius: Length,
    pub border_top_right_radius: Length,
    pub border_top_width: Length,
    pub bottom: Unit,
    pub child_between: Unit,
    pub child_bottom: Unit,
    pub child_left: Unit,
    pub child_right: Unit,
    pub child_top: Unit,
    pub display: Option<Direction>,
    pub flex_basis: Length,
    pub font_family: Option<Arc<str>>,
    pub font_size: f32,
    pub font_style: parley::style::FontStyle,
    pub font_weight: f32,
    pub font_width: f32,
    pub height: Unit,
    pub left: Unit,
    pub letter_spacing: Option<Unit>,
    pub line_height: Unit,
    pub max_bottom: Option<Length>,
    pub max_child_between: Option<Length>,
    pub max_child_bottom: Option<Length>,
    pub max_child_left: Option<Length>,
    pub max_child_right: Option<Length>,
    pub max_child_top: Option<Length>,
    pub max_height: Option<Length>,
    pub max_left: Option<Length>,
    pub max_right: Option<Length>,
    pub max_top: Option<Length>,
    pub max_width: Option<Length>,
    pub min_bottom: Option<Length>,
    pub min_child_between: Option<Length>,
    pub min_child_bottom: Option<Length>,
    pub min_child_left: Option<Length>,
    pub min_child_right: Option<Length>,
    pub min_child_top: Option<Length>,
    pub min_height: Option<Length>,
    pub min_left: Option<Length>,
    pub min_right: Option<Length>,
    pub min_top: Option<Length>,
    pub min_width: Option<Length>,
    pub position: Position,
    pub right: Unit,
    pub text_align: TextAlign,
    pub top: Unit,
    pub width: Unit,
    pub word_spacing: Option<Unit>,
}

// ---------- Style ----------

/// Computed style properties of a Node.
///
/// The [`Ui::on_style`](crate::tree::Ui::on_style) callback gives an application the opportunity to modify a node's style after CSS rules have been applied and before rendering.
#[derive(Clone, PartialEq)]
pub struct Style {
    pub background_color: peniko::Color,
    pub background_image: Option<GradientStack>,
    pub border_bottom_color: peniko::Color,
    pub border_bottom_left_radius: Length,
    pub border_bottom_right_radius: Length,
    pub border_bottom_width: Length,
    pub border_left_color: peniko::Color,
    pub border_left_width: Length,
    pub border_right_color: peniko::Color,
    pub border_right_width: Length,
    pub border_top_color: peniko::Color,
    pub border_top_left_radius: Length,
    pub border_top_right_radius: Length,
    pub border_top_width: Length,
    pub bottom: Unit,
    pub box_shadow: Option<Arc<[BoxShadow]>>,
    pub child_between: Unit,
    pub child_bottom: Unit,
    pub child_left: Unit,
    pub child_right: Unit,
    pub child_top: Unit,
    pub color: peniko::Color,
    pub display: Option<Direction>,
    pub flex_basis: Length,
    pub font_family: Option<Arc<str>>,
    pub font_size: f32,
    pub font_style: parley::style::FontStyle,
    pub font_weight: f32,
    pub font_width: f32,
    pub height: Unit,
    pub left: Unit,
    pub letter_spacing: Option<Unit>,
    pub line_height: Unit,
    pub max_bottom: Option<Length>,
    pub max_child_between: Option<Length>,
    pub max_child_bottom: Option<Length>,
    pub max_child_left: Option<Length>,
    pub max_child_right: Option<Length>,
    pub max_child_top: Option<Length>,
    pub max_height: Option<Length>,
    pub max_left: Option<Length>,
    pub max_right: Option<Length>,
    pub max_top: Option<Length>,
    pub max_width: Option<Length>,
    pub min_bottom: Option<Length>,
    pub min_child_between: Option<Length>,
    pub min_child_bottom: Option<Length>,
    pub min_child_left: Option<Length>,
    pub min_child_right: Option<Length>,
    pub min_child_top: Option<Length>,
    pub min_height: Option<Length>,
    pub min_left: Option<Length>,
    pub min_right: Option<Length>,
    pub min_top: Option<Length>,
    pub min_width: Option<Length>,
    pub opacity: f32,
    pub outline_color: peniko::Color,
    pub outline_offset: Length,
    pub outline_width: Length,
    pub position: Position,
    pub right: Unit,
    pub selection_background: peniko::Color,

    /// If `selection_color` is `None`, selected text will not have a different color.
    pub selection_color: Option<peniko::Color>,
    pub text_align: TextAlign,
    pub text_shadow: Option<Arc<[TextShadow]>>,
    pub top: Unit,
    pub transform: Affine,
    pub visibility: bool,
    pub width: Unit,
    pub word_spacing: Option<Unit>,
    pub z_index: i32,
}

impl std::fmt::Debug for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = Style::default();
        let mut s = f.debug_struct("Style");

        macro_rules! diff {
            ($field:ident) => {
                if self.$field != d.$field {
                    s.field(stringify!($field), &self.$field);
                }
            };
        }

        // peniko::Color -> "#RRGGBBAA" in sRGB (via to_rgba8)
        macro_rules! diff_color {
            ($field:ident) => {
                if self.$field != d.$field {
                    let c = self.$field.to_rgba8();
                    s.field(stringify!($field), &format!("#{:02X}{:02X}{:02X}{:02X}", c.r, c.g, c.b, c.a));
                }
            };
        }

        macro_rules! diff_opt_color {
            ($field:ident) => {
                if self.$field != d.$field {
                    match self.$field {
                        None => s.field(stringify!($field), &"None"),
                        Some(c) => {
                            let c = c.to_rgba8();
                            s.field(stringify!($field), &format!("Some(#{:02X}{:02X}{:02X}{:02X})", c.r, c.g, c.b, c.a))
                        }
                    };
                }
            };
        }

        diff_color!(background_color);
        diff_color!(border_bottom_color);
        diff_color!(border_left_color);
        diff_color!(border_right_color);
        diff_color!(border_top_color);
        diff_color!(color);
        diff_color!(outline_color);
        diff_color!(selection_background);
        diff_opt_color!(selection_color);
        diff!(background_image);
        diff!(border_bottom_left_radius);
        diff!(border_bottom_right_radius);
        diff!(border_bottom_width);
        diff!(border_left_width);
        diff!(border_right_width);
        diff!(border_top_left_radius);
        diff!(border_top_right_radius);
        diff!(border_top_width);
        diff!(bottom);
        diff!(box_shadow);
        diff!(child_between);
        diff!(child_bottom);
        diff!(child_left);
        diff!(child_right);
        diff!(child_top);
        diff!(display);
        diff!(flex_basis);
        diff!(font_family);
        diff!(font_size);
        diff!(font_style);
        diff!(font_weight);
        diff!(font_width);
        diff!(height);
        diff!(left);
        diff!(letter_spacing);
        diff!(line_height);
        diff!(max_bottom);
        diff!(max_child_between);
        diff!(max_child_bottom);
        diff!(max_child_left);
        diff!(max_child_right);
        diff!(max_child_top);
        diff!(max_height);
        diff!(max_left);
        diff!(max_right);
        diff!(max_top);
        diff!(max_width);
        diff!(min_bottom);
        diff!(min_child_between);
        diff!(min_child_bottom);
        diff!(min_child_left);
        diff!(min_child_right);
        diff!(min_child_top);
        diff!(min_height);
        diff!(min_left);
        diff!(min_right);
        diff!(min_top);
        diff!(min_width);
        diff!(opacity);
        diff!(outline_offset);
        diff!(outline_width);
        diff!(position);
        diff!(right);
        diff!(text_align);
        diff!(text_shadow);
        diff!(top);
        diff!(transform);
        diff!(visibility);
        diff!(width);
        diff!(word_spacing);
        diff!(z_index);

        s.finish()
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background_color: peniko::Color::from_rgba8(0, 0, 0, 0),
            background_image: None,
            border_bottom_color: peniko::Color::from_rgba8(0, 0, 0, 255),
            border_bottom_left_radius: Length::Px(0.0),
            border_bottom_right_radius: Length::Px(0.0),
            border_bottom_width: Length::Px(0.0),
            border_left_color: peniko::Color::from_rgba8(0, 0, 0, 255),
            border_left_width: Length::Px(0.0),
            border_right_color: peniko::Color::from_rgba8(0, 0, 0, 255),
            border_right_width: Length::Px(0.0),
            border_top_color: peniko::Color::from_rgba8(0, 0, 0, 255),
            border_top_left_radius: Length::Px(0.0),
            border_top_right_radius: Length::Px(0.0),
            border_top_width: Length::Px(0.0),
            bottom: Unit::Auto,
            box_shadow: None,
            child_between: Unit::Auto,
            child_bottom: Unit::Auto,
            child_left: Unit::Auto,
            child_right: Unit::Auto,
            child_top: Unit::Auto,
            color: peniko::Color::from_rgba8(0, 0, 0, 255),
            display: Some(Direction::Column),
            flex_basis: Length::Px(0.0),
            font_family: None,
            font_size: 16.0,
            font_style: parley::style::FontStyle::Normal,
            font_weight: 400.0,
            font_width: 1.0,
            height: Unit::Stretch(1.0),
            left: Unit::Auto,
            letter_spacing: None,
            line_height: Unit::Stretch(1.2),
            max_bottom: None,
            max_child_between: None,
            max_child_bottom: None,
            max_child_left: None,
            max_child_right: None,
            max_child_top: None,
            max_height: None,
            max_left: None,
            max_right: None,
            max_top: None,
            max_width: None,
            min_bottom: None,
            min_child_between: None,
            min_child_bottom: None,
            min_child_left: None,
            min_child_right: None,
            min_child_top: None,
            min_height: None,
            min_left: None,
            min_right: None,
            min_top: None,
            min_width: None,
            opacity: 1.0,
            outline_color: peniko::Color::from_rgba8(0, 0, 0, 255),
            outline_offset: Length::Px(0.0),
            outline_width: Length::Px(0.0),
            position: Position::ParentDirected,
            right: Unit::Auto,
            selection_background: peniko::Color::from_rgba8(4, 101, 175, 128),
            selection_color: None,
            text_align: TextAlign::Start,
            text_shadow: None,
            top: Unit::Auto,
            transform: Affine::IDENTITY,
            visibility: true,
            width: Unit::Stretch(1.0),
            word_spacing: None,
            z_index: 0,
        }
    }
}

impl Style {
    pub fn get_font_layout_style(&self) -> FontLayoutStyle {
        FontLayoutStyle {
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            font_style: self.font_style,
            font_weight: self.font_weight,
            font_width: self.font_width,
            letter_spacing: self.letter_spacing,
            line_height: self.line_height,
            text_align: self.text_align,
            word_spacing: self.word_spacing,
        }
    }

    pub(crate) fn get_layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            border_bottom_left_radius: self.border_bottom_left_radius,
            border_bottom_right_radius: self.border_bottom_right_radius,
            border_bottom_width: self.border_bottom_width,
            border_left_width: self.border_left_width,
            border_right_width: self.border_right_width,
            border_top_left_radius: self.border_top_left_radius,
            border_top_right_radius: self.border_top_right_radius,
            border_top_width: self.border_top_width,
            bottom: self.bottom,
            child_between: self.child_between,
            child_bottom: self.child_bottom,
            child_left: self.child_left,
            child_right: self.child_right,
            child_top: self.child_top,
            display: self.display,
            flex_basis: self.flex_basis,
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            font_style: self.font_style,
            font_weight: self.font_weight,
            font_width: self.font_width,
            height: self.height,
            left: self.left,
            letter_spacing: self.letter_spacing,
            line_height: self.line_height,
            max_bottom: self.max_bottom,
            max_child_between: self.max_child_between,
            max_child_bottom: self.max_child_bottom,
            max_child_left: self.max_child_left,
            max_child_right: self.max_child_right,
            max_child_top: self.max_child_top,
            max_height: self.max_height,
            max_left: self.max_left,
            max_right: self.max_right,
            max_top: self.max_top,
            max_width: self.max_width,
            min_bottom: self.min_bottom,
            min_child_between: self.min_child_between,
            min_child_bottom: self.min_child_bottom,
            min_child_left: self.min_child_left,
            min_child_right: self.min_child_right,
            min_child_top: self.min_child_top,
            min_height: self.min_height,
            min_left: self.min_left,
            min_right: self.min_right,
            min_top: self.min_top,
            min_width: self.min_width,
            position: self.position,
            right: self.right,
            text_align: self.text_align,
            top: self.top,
            width: self.width,
            word_spacing: self.word_spacing,
        }
    }
}

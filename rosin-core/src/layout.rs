//! # Layout Guide
//!
//! Rosin's layout system is driven by the CSS properties of each node. Layout is solved from the top down,
//! with parents determining the size and position of their children.
//! This means that the final size of a node cannot depend on the size of its children.
//! This constraint allows the layout to be solved more efficiently.
//!
//! ## Layout Value Types
//!
//! - `auto`
//!   - For size properties, `auto` means the node uses its intrinsic size, such as text size or the result of an [`on_measure`](Ui::on_measure) callback.
//!   - For spacing properties, `auto` means use the matching child spacing from the parent.
//!
//! - `<length>`
//!   - `px` is an absolute size.
//!   - `em` is relative to the node's `font-size`.
//!
//! - `<percentage>`
//!   - Relative to the parent's available size along that axis.
//!
//! - `<stretch>`
//!   - A number that takes a proportional share of the remaining free space along the parent's main axis.
//!     Larger numbers get more of that remaining space.
//!
//! Size and spacing can be limited with the matching `min-*` and `max-*` properties.
//!
//! ## Flow Direction
//!
//! The `display` CSS property controls how a node lays out its children:
//!
//! - `none`: The node and its subtree are not laid out.
//! - `row` and `row-reverse`: Children are placed in a horizontal line.
//! - `column` and `column-reverse`: Children are placed in a vertical line.
//!
//! ## Flow Participation
//!
//! The `position` CSS property controls whether a node participates in its parent's flow:
//!
//! - `parent-directed`: The node is part of the parent's flow. It takes space and affects where later siblings land.
//! - `self-directed`: The node is laid out relative to the parent's content origin, but it does not take space in the flow.
//! - `fixed`: The node is laid out relative to the viewport.
//!
//! ## Size
//!
//! The `width` and `height` CSS properties set the node's outer size along each axis.
//! Border width is included, similar to `box-sizing: border-box` in standard CSS.
//!
//! Related constraints:
//!
//! - `min-width`, `max-width`, `min-height`, `max-height`
//!
//! ## Basis
//!
//! The `flex-basis` CSS property is a base size used with `<stretch>` on the main axis. Think of it as the starting point before remaining
//! space is divided up. If the node is not using `<stretch>` on that axis, `flex-basis` has no effect.
//!
//! ## Spacing
//!
//! The `left`, `right`, `top`, and `bottom` CSS properties define extra space around a node in the parent's flow, similar to margins in standard CSS.
//! 
//! The `space` shorthand property sets all four values.
//!
//! - If neither side is set on an axis, the spacing defaults to 0.
//! - If the spacing is set on only one side of an axis, the node will be positioned to satisfy that side.
//! - If both sides on an axis are specified, the "before" side wins: `left` wins over `right`, and `top` wins over `bottom`.
//!
//! Related constraints:
//!
//! - `min-left`, `max-left`, `min-right`, `max-right`, `min-top`, `max-top`, `min-bottom`, `max-bottom`
//!
//! ## Parent Spacing
//!
//! Parents can provide default spacing for their children. Children can override it by setting their own spacing.
//! If the child's side is `auto`, the parent value is used.
//!
//! The `child-left`, `child-right`, `child-top`, and `child-bottom` CSS properties define default spacing applied
//! at the outer edges of each child, similar to padding in standard CSS.
//! 
//! The `child-space` shorthand property sets all four values.
//!
//! Related constraints:
//!
//! - `min-child-left`, `max-child-left`, `min-child-right`, `max-child-right`, `min-child-top`, `max-child-top`, `min-child-bottom`, `max-child-bottom`
//!
//! The `child-between` CSS property defines default spacing inserted between adjacent in-flow children when they
//! do not set their own spacing. This is useful for consistent gaps between siblings.
//!
//! Related constraints:
//!
//! - `min-child-between`, `max-child-between`
//!
//! If both the parent and the child provide constraints, the effective minimum is the larger one and the
//! effective maximum is the smaller one.
//!
//! ## Borders
//!
//! Border widths are part of the node's final size. If you set width/height, the border is counted inside that size.
//! If size is auto, the border is added on top of the measured inner size to produce the outer size.

use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;

use bumpalo::{Bump, collections::Vec as BumpVec};
use kurbo::{Point, Rect, RoundedRect, Shape, Size, Vec2};
use parley::{AlignmentOptions, Layout};

use crate::{hasher::IdentityBuildHasher, prelude::*, text};

// TODO - "position: fixed" items that are trapped by an ancestor's opacity will still be hit-test first
//      - hit_test sorting children may be slow. Might need different strategy

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    Before,
    After,
}

#[inline]
fn resolve_min_opt(min: Option<Length>, font_size: f32) -> f32 {
    match min {
        None => 0.0,
        Some(l) => l.resolve(font_size),
    }
}

#[inline]
fn resolve_max_opt(max: Option<Length>, font_size: f32) -> f32 {
    match max {
        None => f32::INFINITY,
        Some(l) => l.resolve(font_size),
    }
}

#[inline]
fn resolve_radii(style: &Style) -> (f64, f64, f64, f64) {
    (
        style.border_top_left_radius.resolve(style.font_size) as f64,
        style.border_top_right_radius.resolve(style.font_size) as f64,
        style.border_bottom_right_radius.resolve(style.font_size) as f64,
        style.border_bottom_left_radius.resolve(style.font_size) as f64,
    )
}

impl Axis {
    #[inline]
    fn pick<T: Copy>(self, x: T, y: T) -> T {
        match self {
            Axis::X => x,
            Axis::Y => y,
        }
    }

    #[inline]
    fn space(self, edge: Edge, style: &Style) -> Unit {
        match (self, edge) {
            (Axis::X, Edge::Before) => style.left,
            (Axis::X, Edge::After) => style.right,
            (Axis::Y, Edge::Before) => style.top,
            (Axis::Y, Edge::After) => style.bottom,
        }
    }

    #[inline]
    fn min_space(self, edge: Edge, style: &Style) -> Option<Length> {
        match (self, edge) {
            (Axis::X, Edge::Before) => style.min_left,
            (Axis::X, Edge::After) => style.min_right,
            (Axis::Y, Edge::Before) => style.min_top,
            (Axis::Y, Edge::After) => style.min_bottom,
        }
    }

    #[inline]
    fn max_space(self, edge: Edge, style: &Style) -> Option<Length> {
        match (self, edge) {
            (Axis::X, Edge::Before) => style.max_left,
            (Axis::X, Edge::After) => style.max_right,
            (Axis::Y, Edge::Before) => style.max_top,
            (Axis::Y, Edge::After) => style.max_bottom,
        }
    }

    #[inline]
    fn child_space(self, edge: Edge, style: &Style) -> Unit {
        match (self, edge) {
            (Axis::X, Edge::Before) => style.child_left,
            (Axis::X, Edge::After) => style.child_right,
            (Axis::Y, Edge::Before) => style.child_top,
            (Axis::Y, Edge::After) => style.child_bottom,
        }
    }

    #[inline]
    fn min_child_space(self, edge: Edge, style: &Style) -> Option<Length> {
        match (self, edge) {
            (Axis::X, Edge::Before) => style.min_child_left,
            (Axis::X, Edge::After) => style.min_child_right,
            (Axis::Y, Edge::Before) => style.min_child_top,
            (Axis::Y, Edge::After) => style.min_child_bottom,
        }
    }

    #[inline]
    fn max_child_space(self, edge: Edge, style: &Style) -> Option<Length> {
        match (self, edge) {
            (Axis::X, Edge::Before) => style.max_child_left,
            (Axis::X, Edge::After) => style.max_child_right,
            (Axis::Y, Edge::Before) => style.max_child_top,
            (Axis::Y, Edge::After) => style.max_child_bottom,
        }
    }

    #[inline]
    fn add_offset_to(self, vec: &mut Vec2, offset: f64) {
        match self {
            Axis::X => vec.x += offset,
            Axis::Y => vec.y += offset,
        };
    }

    #[inline]
    fn set_size_component(self, size: &mut Size, value: f64) {
        match self {
            Axis::X => size.width = value,
            Axis::Y => size.height = value,
        }
    }

    #[inline]
    fn basis(self, style: &Style, measure: Option<Size>, main_axis: bool) -> f32 {
        match self.pick(style.width, style.height) {
            Unit::Auto => measure.map_or(0.0, |m| self.pick(m.width, m.height) as f32),
            _ if main_axis => style.flex_basis.resolve(style.font_size),
            _ => 0.0,
        }
    }
}

#[inline]
fn pick_non_auto(primary: Unit, fallback: Unit) -> Unit {
    if primary != Unit::Auto { primary } else { fallback }
}

#[derive(Default)]
struct StretchItem {
    idx: Option<NonZeroUsize>,
    display_none: bool,
    position: Position,

    font_size: f32,
    size: Unit,
    basis: f32,
    min_size: f32,
    max_size: f32,

    border_before: f32,
    border_after: f32,

    frozen: bool,
    violation: f32,
    target: f32,

    measure: Option<Size>,
    solved_size: Size,
    solved_offset: Vec2,
}

impl StretchItem {
    #[inline]
    fn new(idx: usize, style: &Style, axis: Axis, measure: Option<Size>, basis: Option<f32>) -> Self {
        debug_assert!(idx != 0);

        let min_opt = axis.pick(style.min_width, style.min_height);
        let max_opt = axis.pick(style.max_width, style.max_height);

        let min_size = resolve_min_opt(min_opt, style.font_size);
        let max_size = resolve_max_opt(max_opt, style.font_size);

        let border_before = match axis {
            Axis::X => style.border_left_width.resolve(style.font_size),
            Axis::Y => style.border_top_width.resolve(style.font_size),
        };
        let border_after = match axis {
            Axis::X => style.border_right_width.resolve(style.font_size),
            Axis::Y => style.border_bottom_width.resolve(style.font_size),
        };

        Self {
            idx: NonZeroUsize::new(idx),
            display_none: style.display.is_none(),
            position: style.position,
            font_size: style.font_size,
            size: axis.pick(style.width, style.height),
            basis: basis.unwrap_or(style.flex_basis.resolve(style.font_size)),
            min_size,
            max_size,
            border_before,
            border_after,
            frozen: false,
            violation: 0.0,
            target: 0.0,
            measure,
            solved_size: Size::ZERO,
            solved_offset: Vec2::ZERO,
        }
    }

    #[inline]
    fn new_space(size: Unit, min_size: f32, max_size: f32, font_size: f32) -> Self {
        StretchItem {
            idx: None,
            display_none: false,
            position: Position::ParentDirected,
            font_size,
            size,
            basis: 0.0,
            min_size,
            max_size,
            border_before: 0.0,
            border_after: 0.0,
            frozen: false,
            violation: 0.0,
            target: 0.0,
            measure: None,
            solved_size: Size::ZERO,
            solved_offset: Vec2::ZERO,
        }
    }

    #[inline]
    fn edge_space(axis: Axis, edge: Edge, child: &Style, parent: &Style) -> StretchItem {
        // em spacing should use the font size of the node that won.
        let child_u = axis.space(edge, child);
        let parent_u = axis.child_space(edge, parent);

        let (unit, font_size) = if child_u != Unit::Auto {
            (child_u, child.font_size)
        } else if parent_u != Unit::Auto {
            (parent_u, parent.font_size)
        } else {
            (Unit::default(), parent.font_size)
        };

        let min_child = resolve_min_opt(axis.min_space(edge, child), font_size);
        let min_parent = resolve_min_opt(axis.min_child_space(edge, parent), font_size);
        let max_child = resolve_max_opt(axis.max_space(edge, child), font_size);
        let max_parent = resolve_max_opt(axis.max_child_space(edge, parent), font_size);

        StretchItem::new_space(unit, min_child.max(min_parent), max_child.min(max_parent), font_size)
    }

    #[inline]
    fn between_space(axis: Axis, prev: &Style, next: &Style, parent: &Style) -> StretchItem {
        // em spacing should use the font size of the node that won.
        let prev_u = axis.space(Edge::After, prev);
        let next_u = axis.space(Edge::Before, next);
        let parent_u = parent.child_between;

        let (unit, font_size) = if prev_u != Unit::Auto {
            (prev_u, prev.font_size)
        } else if next_u != Unit::Auto {
            (next_u, next.font_size)
        } else if parent_u != Unit::Auto {
            (parent_u, parent.font_size)
        } else {
            (Unit::default(), parent.font_size)
        };

        let min_prev = resolve_min_opt(axis.min_space(Edge::After, prev), font_size);
        let min_next = resolve_min_opt(axis.min_space(Edge::Before, next), font_size);
        let min_parent = resolve_min_opt(parent.min_child_between, font_size);

        let max_prev = resolve_max_opt(axis.max_space(Edge::After, prev), font_size);
        let max_next = resolve_max_opt(axis.max_space(Edge::Before, next), font_size);
        let max_parent = resolve_max_opt(parent.max_child_between, font_size);

        StretchItem::new_space(unit, min_prev.max(min_next).max(min_parent), max_prev.min(max_next).min(max_parent), font_size)
    }
}

pub(crate) struct TextCacheEntry {
    pub deps: DependencyMap,
    pub layout: Layout<[u8; 4]>,
    pub font_style: FontLayoutStyle,
    pub max_width: Option<f32>,
}

impl fmt::Debug for TextCacheEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextCacheEntry")
            .field("deps", &self.deps)
            .field("layout", &"<omitted>")
            .field("font_style", &self.font_style)
            .field("max_width", &self.max_width)
            .finish()
    }
}

struct LayoutCtx<'a, S: 'static> {
    state: &'a S,
    translation_map: &'a TranslationMap,
    text_cache: &'a mut HashMap<usize, TextCacheEntry, IdentityBuildHasher>,
}

#[inline]
fn resolve_unit(unit: Unit, font_size: f32, pct_base: f32, min: f32, max: f32) -> Option<f32> {
    match unit {
        u if u.is_definite() => Some(u.definite_size(font_size, pct_base).min(max).max(min)),
        _ => None,
    }
}

fn collect_single_item_array(style: &Style, parent: &Style, axis: Axis, basis: f32) -> [StretchItem; 3] {
    let mut result = [
        StretchItem::edge_space(axis, Edge::Before, style, parent),
        StretchItem::new(1, style, axis, None, Some(basis)),
        StretchItem::edge_space(axis, Edge::After, style, parent),
    ];
    // This is needed to position fixed and parent-directed nodes that would otherwise be ignored.
    result[1].position = Position::ParentDirected;
    result
}

fn solve(items: &mut [StretchItem], available_space: f32) {
    // Freeze inflexible items
    for item in items.iter_mut() {
        if item.display_none || item.position != Position::ParentDirected {
            item.target = 0.0;
            item.frozen = true;
            continue;
        }

        let (frozen, raw_value) = match item.size {
            Unit::Stretch(_) => (false, item.basis),
            Unit::Auto => (true, item.basis),
            _ => (true, resolve_unit(item.size, item.font_size, available_space, item.min_size, item.max_size).unwrap_or(item.basis)),
        };

        item.frozen = frozen;
        item.target = if frozen { raw_value.min(item.max_size).max(item.min_size) } else { raw_value };
    }

    loop {
        // Check for inflexible items
        if items.iter().all(|item| item.frozen) {
            break;
        }

        // Calculate remaining free space
        let mut used_space = 0.0;
        for item in items.iter().filter(|item| !item.display_none && item.position == Position::ParentDirected) {
            used_space += if item.frozen {
                item.target + item.border_before + item.border_after
            } else {
                item.basis + item.border_before + item.border_after
            };
        }
        let free_space = available_space - used_space;

        // Distribute free space proportionally
        let mut total_s = 0.0;
        for item in items.iter().filter(|item| !item.frozen) {
            if let Unit::Stretch(s) = item.size {
                total_s += s;
            }
        }
        if total_s == 0.0 {
            for item in items.iter_mut().filter(|i| !i.frozen) {
                item.target = item.target.min(item.max_size).max(item.min_size);
                item.frozen = true;
            }
            break;
        }
        let size_per_s = free_space / total_s;
        for item in items.iter_mut().filter(|item| !item.frozen) {
            if let Unit::Stretch(s) = item.size {
                item.target = item.basis + (s * size_per_s);
            }
        }

        // Fix min/max violations
        let mut total_violation = 0.0;
        for item in items.iter_mut().filter(|item| !item.frozen) {
            let prev_target = item.target;
            item.target = prev_target.min(item.max_size).max(item.min_size);
            item.violation = item.target - prev_target;
            total_violation += item.violation;
        }

        // Freeze over-flexed items
        for item in items.iter_mut().filter(|item| !item.frozen) {
            match total_violation {
                v if v > 0.0 => item.frozen = item.violation > 0.0,
                v if v < 0.0 => item.frozen = item.violation < 0.0,
                _ => item.frozen = true,
            }
        }
    }
}

fn solve_axis(item: &mut StretchItem, style: &Style, parent: &Style, axis: Axis, available: f32, basis: f32) {
    let mut axis_items = collect_single_item_array(style, parent, axis, basis);
    solve(&mut axis_items, available);

    let outer = axis_items[1].target + axis_items[1].border_before + axis_items[1].border_after;
    axis.set_size_component(&mut item.solved_size, outer as f64);

    let before_unit = pick_non_auto(axis.space(Edge::Before, style), axis.child_space(Edge::Before, parent));
    let after_unit = pick_non_auto(axis.space(Edge::After, style), axis.child_space(Edge::After, parent));

    let before_constrained = (before_unit != Unit::Auto) || axis_items[0].min_size > 0.0;
    let after_constrained = (after_unit != Unit::Auto) || axis_items[2].min_size > 0.0;

    let offset = if !before_constrained && after_constrained {
        available - axis_items[2].target - outer
    } else {
        axis_items[0].target
    };

    axis.add_offset_to(&mut item.solved_offset, offset as f64);
}

/// This should only be used to lay out text if it has size auto.
/// All other text should be laid out in draw() to avoid unnecessarily registering dependencies to the layout phase.
///
/// max_size refers to the node's border box
///
/// returns padding box
fn measure_node<S, H>(ctx: &mut LayoutCtx<'_, S>, tree: &'_ Ui<S, H>, idx: usize, max_size: Size, pct_size: Size, intrinsic: bool) -> Option<Size> {
    let style = &tree.style_cache[idx];
    if style.width != Unit::Auto && style.height != Unit::Auto {
        return None;
    }

    let parent_width = pct_size.width as f32;
    let parent_height = pct_size.height as f32;

    #[inline]
    fn resolve_pad_for_measure(unit: Unit, font_size: f32, pct_base: f32, min: f32, max: f32) -> f32 {
        match unit {
            u if u.is_definite() => u.definite_size(font_size, pct_base).min(max).max(min),
            _ => min.min(max),
        }
    }

    let min_top = resolve_min_opt(style.min_child_top, style.font_size);
    let max_top = resolve_max_opt(style.max_child_top, style.font_size);
    let min_right = resolve_min_opt(style.min_child_right, style.font_size);
    let max_right = resolve_max_opt(style.max_child_right, style.font_size);
    let min_left = resolve_min_opt(style.min_child_left, style.font_size);
    let max_left = resolve_max_opt(style.max_child_left, style.font_size);
    let min_bottom = resolve_min_opt(style.min_child_bottom, style.font_size);
    let max_bottom = resolve_max_opt(style.max_child_bottom, style.font_size);

    let pad_top = resolve_pad_for_measure(style.child_top, style.font_size, parent_height, min_top, max_top);
    let pad_right = resolve_pad_for_measure(style.child_right, style.font_size, parent_width, min_right, max_right);
    let pad_left = resolve_pad_for_measure(style.child_left, style.font_size, parent_width, min_left, max_left);
    let pad_bottom = resolve_pad_for_measure(style.child_bottom, style.font_size, parent_height, min_bottom, max_bottom);

    let pad_size = Size::new((pad_left + pad_right) as f64, (pad_top + pad_bottom) as f64);

    let border_left = style.border_left_width.resolve(style.font_size);
    let border_right = style.border_right_width.resolve(style.font_size);
    let border_top = style.border_top_width.resolve(style.font_size);
    let border_bottom = style.border_bottom_width.resolve(style.font_size);

    let border_width = border_left + border_right;
    let border_height = border_top + border_bottom;
    let border_size = Size::new(border_width as f64, border_height as f64);

    let text_size = if let Some(text) = &tree.nodes[idx].text {
        let font_style = style.get_font_layout_style();
        let max_width = (!intrinsic).then(|| (max_size.width as f32 - border_width - pad_left - pad_right).max(0.0));

        let mut from_cache: Option<Size> = None;
        if let Some(cache) = ctx.text_cache.get_mut(&idx) {
            // even if we don't use the cache, we still depend on the vars
            cache.deps.mark_read();

            // reuse cache if still valid
            if !cache.deps.any_changed_update() && cache.font_style == font_style {
                cache.layout.break_all_lines(max_width);
                cache.max_width = max_width;

                from_cache = Some(Size {
                    width: cache.layout.width() as f64,
                    height: cache.layout.height() as f64,
                });
            }
        }

        from_cache.unwrap_or_else(|| {
            // resolve text, layout, cache
            let mut final_layout = None;
            let deps = DependencyMap::default().read_scope(|| {
                if let Some(resolved) = text.resolve(ctx.translation_map) {
                    let mut layout = text::layout_text(&font_style, max_width, &resolved);
                    layout.break_all_lines(max_width);
                    final_layout = Some(layout);
                }
            });

            if let Some(layout) = final_layout {
                let size = Size {
                    width: layout.width() as f64,
                    height: layout.height() as f64,
                };

                ctx.text_cache.insert(
                    idx,
                    TextCacheEntry {
                        deps,
                        layout,
                        font_style,
                        max_width,
                    },
                );
                size
            } else {
                Size::ZERO
            }
        })
    } else {
        Size::ZERO
    };

    if let Some(measure_callback) = &tree.nodes[idx].measure_callback {
        let measure_ctx = MeasureCtx {
            style,
            max_size: (!intrinsic).then_some(max_size),
        };
        let border_box = measure_callback(ctx.state, &measure_ctx);
        Some(if border_box.is_finite() { border_box } else { max_size } - border_size)
    } else {
        Some(text_size + pad_size + Size::new(1.0, 0.0)) // Add a pixel to text width in case the box shrinks during rounding.
    }
}

#[inline]
pub(crate) fn layout<S, H>(state: &S, temp: &Bump, tree: &mut Ui<S, H>, viewport_size: Size, scale: Vec2, translation_map: &TranslationMap) {
    debug_assert!(scale.x > 0.0 && scale.y > 0.0);

    tree.fixed_nodes.clear();
    tree.layout_cache.clear();
    tree.layout_cache.reserve(tree.nodes.len());

    let (rtl, rtr, rbr, rbl) = resolve_radii(&tree.style_cache[0]);
    let root_rect = Rect::ZERO.with_size(viewport_size).to_rounded_rect((rtl, rtr, rbr, rbl));

    let mut text_cache = std::mem::take(&mut tree.text_cache);
    let mut stack: BumpVec<(usize, RoundedRect, bool)> = BumpVec::with_capacity_in(tree.nodes.len(), temp);
    stack.push((0, root_rect, false));

    // Allocate enough space for the worst case scenario
    let mut out_stack: BumpVec<(usize, RoundedRect, bool)> = BumpVec::with_capacity_in(tree.max_children, temp);
    let mut children: BumpVec<usize> = BumpVec::with_capacity_in(tree.max_children, temp);
    let mut stretch_items: BumpVec<StretchItem> = BumpVec::with_capacity_in(tree.max_children * 2 + 1, temp);

    while let Some((parent_idx, parent_rect, display_none)) = stack.pop() {
        tree.layout_cache.push(parent_rect);

        tree.child_indexes(parent_idx, &mut children);
        if children.is_empty() {
            continue;
        }

        let parent_style = &tree.style_cache[parent_idx];
        let (Some(dir), false) = (parent_style.display, display_none) else {
            // display: none - just push children onto stack and continue
            for child_idx in children.iter().rev() {
                stack.push((*child_idx, Rect::ZERO.to_rounded_rect(0.0), true));
            }
            continue;
        };

        if dir.is_reverse() {
            children.reverse();
        }

        let main_axis = if dir.is_row() { Axis::X } else { Axis::Y };
        let cross_axis = if dir.is_row() { Axis::Y } else { Axis::X };

        let parent_w = parent_rect.width() as f32;
        let parent_h = parent_rect.height() as f32;

        let parent_border_left = parent_style.border_left_width.resolve(parent_style.font_size);
        let parent_border_right = parent_style.border_right_width.resolve(parent_style.font_size);
        let parent_border_top = parent_style.border_top_width.resolve(parent_style.font_size);
        let parent_border_bottom = parent_style.border_bottom_width.resolve(parent_style.font_size);

        let parent_padding_box_w = (parent_w - parent_border_left - parent_border_right).max(0.0);
        let parent_padding_box_h = (parent_h - parent_border_top - parent_border_bottom).max(0.0);
        let parent_padding_box_size = Size::new(parent_padding_box_w as f64, parent_padding_box_h as f64);

        let available_main = main_axis.pick(parent_padding_box_w, parent_padding_box_h);
        let available_cross = cross_axis.pick(parent_padding_box_w, parent_padding_box_h);

        let mut ctx = LayoutCtx {
            state,
            translation_map,
            text_cache: &mut text_cache,
        };

        // ---------- Collect StretchItems ----------

        stretch_items.clear();

        let mut prev_affecting_idx: Option<usize> = None;
        for &idx in children.iter() {
            let style = &tree.style_cache[idx];
            let affecting = style.display.is_some() && style.position == Position::ParentDirected;

            // Add space right before the next affecting item
            if affecting {
                if let Some(prev_idx) = prev_affecting_idx {
                    let prev_style = &tree.style_cache[prev_idx];
                    stretch_items.push(StretchItem::between_space(main_axis, prev_style, style, parent_style));
                } else {
                    stretch_items.push(StretchItem::edge_space(main_axis, Edge::Before, style, parent_style));
                }
            }

            let measure = if style.display.is_some() && style.width == Unit::Auto {
                let max_size = match style.position {
                    Position::Fixed => viewport_size,
                    _ => parent_padding_box_size,
                };

                measure_node(&mut ctx, tree, idx, max_size, max_size, dir.is_row())
            } else {
                None
            };

            let basis = if dir.is_row() && style.position == Position::ParentDirected && style.width == Unit::Auto {
                measure.map(|m| m.width as f32)
            } else {
                None
            };

            // Add item
            let mut item = StretchItem::new(idx, style, main_axis, measure, basis);

            if !dir.is_row() && style.display.is_some() && item.position == Position::ParentDirected {
                // Solve width first for parent-directed nodes so we can wrap text to
                // the final content width and compute the correct Auto height basis.
                let basis_x = if style.width == Unit::Auto {
                    item.measure.map(|m| m.width as f32).unwrap_or(0.0)
                } else {
                    0.0
                };

                solve_axis(&mut item, style, parent_style, Axis::X, available_cross, basis_x);

                // If height is Auto, compute the correct wrapped height basis now.
                if style.height == Unit::Auto {
                    let constrained = Size::new(item.solved_size.width, parent_padding_box_h as f64);
                    item.measure = measure_node(&mut ctx, tree, idx, constrained, parent_padding_box_size, false);

                    if let Some(measured) = item.measure {
                        item.basis = measured.height as f32;
                    }
                }
            }

            stretch_items.push(item);

            if affecting {
                prev_affecting_idx = Some(idx);
            }
        }

        // Add trailing edge space after the last affecting child
        if let Some(last_idx) = prev_affecting_idx {
            let last_style = &tree.style_cache[last_idx];
            stretch_items.push(StretchItem::edge_space(main_axis, Edge::After, last_style, parent_style));
        }

        // ---------- Calculate Sizes ----------

        solve(&mut stretch_items, available_main);

        // Finalize size
        for item in &mut stretch_items {
            let Some(idx) = item.idx.map(|n| n.get()) else {
                continue;
            };
            if item.display_none {
                continue;
            }

            let style = &tree.style_cache[idx];

            match item.position {
                Position::ParentDirected => {
                    // Main axis is already solved in item.target
                    let outer_main = item.target + item.border_before + item.border_after;
                    main_axis.set_size_component(&mut item.solved_size, outer_main as f64);

                    if !dir.is_row() {
                        // Column parent-directed items already had their width solved during pre-pass.
                        continue;
                    }

                    if style.height == Unit::Auto {
                        // Re-measure if needed
                        let constrained = Size::new(item.solved_size.width, parent_padding_box_h as f64);
                        item.measure = measure_node(&mut ctx, tree, idx, constrained, parent_padding_box_size, false);
                    }

                    // Cross axis
                    let cross_basis = cross_axis.basis(style, item.measure, false);
                    solve_axis(item, style, parent_style, cross_axis, available_cross, cross_basis);
                }
                Position::SelfDirected => {
                    // Main Axis
                    let basis_x = Axis::X.basis(style, item.measure, main_axis == Axis::X);
                    solve_axis(item, style, &Style::default(), Axis::X, parent_padding_box_w, basis_x);

                    if style.height == Unit::Auto {
                        // Re-measure if needed
                        let constrained = Size::new(item.solved_size.width, parent_padding_box_h as f64);
                        item.measure = measure_node(&mut ctx, tree, idx, constrained, parent_padding_box_size, false);
                    }

                    // Cross Axis
                    let basis_y = Axis::Y.basis(style, item.measure, main_axis == Axis::Y);
                    solve_axis(item, style, &Style::default(), Axis::Y, parent_padding_box_h, basis_y);
                }
                Position::Fixed => {
                    // Main Axis
                    let basis_x = Axis::X.basis(style, item.measure, true);
                    solve_axis(item, style, &Style::default(), Axis::X, viewport_size.width as f32, basis_x);

                    if style.height == Unit::Auto {
                        // Re-measure if needed
                        let constrained = Size::new(item.solved_size.width, viewport_size.height);
                        item.measure = measure_node(&mut ctx, tree, idx, constrained, viewport_size, false);
                    }

                    // Cross Axis
                    let basis_y = Axis::Y.basis(style, item.measure, false);
                    solve_axis(item, style, &Style::default(), Axis::Y, viewport_size.height as f32, basis_y);

                    tree.fixed_nodes.push(idx);
                }
            }
        }

        // ---------- Create Rects ----------

        out_stack.clear();

        let initial_pos = parent_rect.origin().to_vec2() + Vec2::new(parent_border_left as f64, parent_border_top as f64);
        let mut current_pos = initial_pos;
        let mut children_bounds: Option<Rect> = None;

        for item in &stretch_items {
            // Spaces only advance the offset.
            let Some(idx) = item.idx else {
                let main_size = item.target + item.border_before + item.border_after;
                main_axis.add_offset_to(&mut current_pos, main_size as f64);
                continue;
            };
            let idx = idx.get();

            if item.display_none {
                // display none items still need a slot in the output list
                out_stack.push((idx, Rect::ZERO.to_rounded_rect(0.0), true));
                continue;
            }

            let origin_base = match item.position {
                Position::ParentDirected => current_pos,
                Position::SelfDirected => initial_pos,
                Position::Fixed => Vec2::ZERO,
            };

            let origin = origin_base + item.solved_offset;

            let (rtl, rtr, rbr, rbl) = resolve_radii(&tree.style_cache[idx]);

            // Build rect from the solved size.
            let rect = Rect::from_origin_size(origin.to_point(), item.solved_size).to_rounded_rect((rtl, rtr, rbr, rbl));

            // Accumulate result into children_bounds
            if item.position != Position::Fixed {
                children_bounds = Some(match children_bounds {
                    Some(b) => b.union(rect.rect()),
                    None => rect.rect(),
                });
            }

            out_stack.push((idx, rect, false));

            // Advance offset for parent-directed items.
            if item.position == Position::ParentDirected {
                let main_size = main_axis.pick(item.solved_size.width, item.solved_size.height);
                main_axis.add_offset_to(&mut current_pos, main_size);
            }
        }

        // Apply offset to children, clamped to children_bounds
        if let Some(node_offset) = tree.nodes[parent_idx].offset.as_mut() {
            node_offset.with_mut(|offset| {
                if let Some(children_bounds) = children_bounds {
                    let content_x0 = parent_rect.origin().x + parent_border_left as f64;
                    let content_y0 = parent_rect.origin().y + parent_border_top as f64;
                    let content_x1 = content_x0 + parent_padding_box_w as f64;
                    let content_y1 = content_y0 + parent_padding_box_h as f64;

                    let min_x = content_x1 - children_bounds.x1;
                    let max_x = content_x0 - children_bounds.x0;
                    let (min_x, max_x) = if min_x < max_x { (min_x, max_x) } else { (max_x, min_x) };
                    offset.x = offset.x.clamp(min_x, max_x);

                    let min_y = content_y1 - children_bounds.y1;
                    let max_y = content_y0 - children_bounds.y0;
                    let (min_y, max_y) = if min_y < max_y { (min_y, max_y) } else { (max_y, min_y) };
                    offset.y = offset.y.clamp(min_y, max_y);

                    for (idx, rect, display_none) in &mut out_stack {
                        if !*display_none && tree.style_cache[*idx].position != Position::Fixed {
                            *rect = Rect::from_origin_size(rect.origin() + *offset, Size::new(rect.width(), rect.height())).to_rounded_rect(rect.radii());
                        }
                    }
                }
            });
        }

        // Round layout to nearest physical pixel
        for (_, rect, display_none) in &mut out_stack {
            if !*display_none {
                let x0p = (rect.origin().x * scale.x).round();
                let y0p = (rect.origin().y * scale.y).round();
                let x1p = ((rect.origin().x + rect.width()) * scale.x).round();
                let y1p = ((rect.origin().y + rect.height()) * scale.y).round();

                let origin_x = x0p / scale.x;
                let origin_y = y0p / scale.y;

                let width = ((x1p - x0p) / scale.x).max(0.0);
                let height = ((y1p - y0p) / scale.y).max(0.0);

                *rect = Rect::from_origin_size(Point::new(origin_x, origin_y), Size::new(width, height)).to_rounded_rect(rect.radii());
            }
        }

        // Need to push items onto stack in reverse order to ensure they get added to the output array in tree order
        if dir.is_reverse() {
            stack.append(&mut out_stack);
        } else {
            stack.extend(out_stack.drain(..).rev());
        }
    }

    // Now that the list is complete, sort fixed_nodes by z-index
    tree.fixed_nodes.sort_by(|a, b| tree.style_cache[*a].z_index.cmp(&tree.style_cache[*b].z_index));

    // Put the cache back
    tree.text_cache = text_cache;
}

/// Aligns text to available space after stretch evaluation and returns the origin in local space.
pub(crate) fn align_and_position_text(style: &Style, rect: &RoundedRect, layout: &mut Layout<[u8; 4]>) -> Point {
    let font_size = style.font_size;

    let padding_box = padding_box(style, rect);
    let padding_box_w = padding_box.width() as f32;
    let padding_box_h = padding_box.height() as f32;

    let text_w = layout.width();
    let text_h = layout.height();

    // Solve X Axis
    let mut x_items = [
        StretchItem::edge_space(Axis::X, Edge::Before, &Style::default(), style),
        StretchItem::new_space(Unit::Auto, 0.0, f32::INFINITY, font_size),
        StretchItem::edge_space(Axis::X, Edge::After, &Style::default(), style),
    ];
    x_items[1].position = Position::ParentDirected;
    x_items[1].basis = text_w;
    solve(&mut x_items, padding_box_w);

    // align text to remaining width after resolving stretch units, if any
    let left = x_items[0].target;
    let right = x_items[2].target;
    let content_w = (padding_box_w - left - right).max(0.0);
    layout.align(Some(content_w), style.text_align.into(), AlignmentOptions::default());

    // Solve Y Axis
    let mut y_items = [
        StretchItem::edge_space(Axis::Y, Edge::Before, &Style::default(), style),
        StretchItem::new_space(Unit::Auto, 0.0, f32::INFINITY, font_size),
        StretchItem::edge_space(Axis::Y, Edge::After, &Style::default(), style),
    ];
    y_items[1].position = Position::ParentDirected;
    y_items[1].basis = text_h;

    solve(&mut y_items, padding_box_h);

    let top = y_items[0].target;

    Point::new(padding_box.x0 + left as f64, padding_box.y0 + top as f64)
}

pub(crate) fn padding_box(style: &Style, rect: &RoundedRect) -> Rect {
    let border_left = style.border_left_width.resolve(style.font_size);
    let border_right = style.border_right_width.resolve(style.font_size);
    let border_top = style.border_top_width.resolve(style.font_size);
    let border_bottom = style.border_bottom_width.resolve(style.font_size);

    let width = (rect.width() - border_left as f64 - border_right as f64).max(0.0);
    let height = (rect.height() - border_top as f64 - border_bottom as f64).max(0.0);

    Rect::from_origin_size((border_left, border_top), (width, height))
}

pub(crate) fn max_content_width(style: &Style, rect: &RoundedRect) -> f32 {
    let width = rect.width() as f32;
    let font_size = style.font_size;

    let border_left = style.border_left_width.resolve(font_size);
    let border_right = style.border_right_width.resolve(font_size);

    let min_l = resolve_min_opt(style.min_child_left, font_size);
    let max_l = resolve_max_opt(style.max_child_left, font_size);
    let min_r = resolve_min_opt(style.min_child_right, font_size);
    let max_r = resolve_max_opt(style.max_child_right, font_size);

    let left = match style.child_left {
        u if u.is_definite() => u.definite_size(font_size, width),
        _ => 0.0,
    }
    .min(max_l)
    .max(min_l);

    let right = match style.child_right {
        u if u.is_definite() => u.definite_size(font_size, width),
        _ => 0.0,
    }
    .min(max_r)
    .max(min_r);

    (width - border_left - border_right - left - right).max(0.0)
}

// output must be sorted
pub(crate) fn hit_test<S, H>(temp: &Bump, tree: &Ui<S, H>, point: Point, output: &mut Vec<usize>) {
    output.clear();

    // First check nodes with "position: fixed" property, and use as a starting point if found
    for &idx in tree.fixed_nodes.iter().rev() {
        if tree.style_cache[idx].display.is_none() {
            continue;
        }

        if tree.layout_cache[idx].contains(point) {
            let mut ancestor = idx;
            while ancestor != 0 {
                output.push(ancestor);
                ancestor = tree.nodes[ancestor].parent;
            }
            output.push(0);
            output.reverse();
            break;
        }
    }

    // Walk down tree adding children that contain the point
    let mut curr: usize = output.pop().unwrap_or(0);
    let mut children: BumpVec<usize> = BumpVec::with_capacity_in(tree.max_children, temp);
    while tree.layout_cache[curr].contains(point) {
        output.push(curr);

        tree.child_indexes(curr, &mut children);
        children.sort_by_key(|&a| tree.style_cache[a].z_index);

        let mut found = false;
        for &child_idx in children.iter().rev() {
            if tree.style_cache[child_idx].display.is_none() {
                continue;
            }

            if tree.layout_cache[child_idx].contains(point) {
                curr = child_idx;
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }
}

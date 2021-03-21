#![forbid(unsafe_code)]

/*
 * Copyright (c) 2018 Visly Inc.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

// Currently unsupported:
//   - percentages
//   - visibility:collapse
//   - cases 3.c and 3.d (https://www.w3.org/TR/css-flexbox-1/#algo-main-item)

use crate::geometry::*;
use crate::style::*;
use crate::tree::ArrayNode;

use bumpalo::{collections::Vec as BumpVec, Bump};

trait FiniteOrElse {
    fn finite_or_else<F: FnOnce() -> Self>(self, func: F) -> Self;
}

impl FiniteOrElse for f32 {
    fn finite_or_else<F: FnOnce() -> f32>(self, func: F) -> f32 {
        if self.is_finite() {
            self
        } else {
            func()
        }
    }
}

impl Style {
    fn cross_size(&self, dir: FlexDirection) -> Option<f32> {
        if !dir.is_row() {
            self.width
        } else {
            self.height
        }
    }

    fn main_margin_start(&self, dir: FlexDirection) -> Option<f32> {
        if dir.is_row() {
            self.margin_left
        } else {
            self.margin_top
        }
    }

    fn main_margin_end(&self, dir: FlexDirection) -> Option<f32> {
        if dir.is_row() {
            self.margin_right
        } else {
            self.margin_bottom
        }
    }

    fn cross_margin_start(&self, dir: FlexDirection) -> Option<f32> {
        if !dir.is_row() {
            self.margin_left
        } else {
            self.margin_top
        }
    }

    fn cross_margin_end(&self, dir: FlexDirection) -> Option<f32> {
        if !dir.is_row() {
            self.margin_right
        } else {
            self.margin_bottom
        }
    }

    fn position(&self) -> Rect {
        Rect {
            top: self.top.unwrap_or(0.0),
            right: self.right.unwrap_or(0.0),
            bottom: self.bottom.unwrap_or(0.0),
            left: self.left.unwrap_or(0.0),
        }
    }

    fn margin(&self) -> Rect {
        Rect {
            top: self.margin_top.unwrap_or(0.0),
            right: self.margin_right.unwrap_or(0.0),
            bottom: self.margin_bottom.unwrap_or(0.0),
            left: self.margin_left.unwrap_or(0.0),
        }
    }

    fn border(&self) -> Rect {
        Rect {
            top: self.border_top_width,
            right: self.border_right_width,
            bottom: self.border_bottom_width,
            left: self.border_left_width,
        }
    }

    fn padding(&self) -> Rect {
        Rect {
            top: self.padding_top,
            right: self.padding_right,
            bottom: self.padding_bottom,
            left: self.padding_left,
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
struct Cache {
    // Either both bounds are infinite, or the main axis has been determined
    inf_result: Option<Size>,
    // TODO - check if bounds ever differs between calls. I think it wont, so we won't need this
    bounds: Size,
    fin_result: Option<Size>,
}

#[derive(Debug)]
struct FlexItem {
    id: usize,

    min_size: Size,
    max_size: Size,

    position: Rect,
    margin: Rect,
    mbp: Rect,

    flex_basis: f32,
    flex_grow: f32,
    flex_shrink: f32,

    violation: f32,
    frozen: bool,

    hypo_outer_size: Size,
    hypo_inner_size: Size,
    target_size: Size,

    baseline: f32,

    offset_main: f32,
    offset_cross: f32,
}

#[derive(Debug)]
struct FlexLine<'a> {
    items: &'a mut [FlexItem],
    cross_size: f32,
    offset_cross: f32,
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct Layout {
    pub size: Size,
    pub position: Point,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            size: Size { width: 0.0, height: 0.0 },
            position: Point { x: 0.0, y: 0.0 },
        }
    }
}

pub(crate) fn build_layout<T>(scratch: &Bump, tree: &[ArrayNode<T>], root_size: Size, output: &mut [Layout]) {
    let mut cache = bumpalo::vec![in &scratch; Cache::default(); tree.len()];
    layout(scratch, tree, 0, root_size, false, &mut cache, output);
    output[0] = Layout {
        size: root_size,
        position: Point::zero(),
    };
    round_layout(tree, output, 0, 0.0, 0.0);
}

fn round_layout<T>(tree: &[ArrayNode<T>], layout: &mut [Layout], id: usize, abs_x: f32, abs_y: f32) {
    let abs_x = abs_x + layout[id].position.x;
    let abs_y = abs_y + layout[id].position.y;

    layout[id].position.x = layout[id].position.x.round();
    layout[id].position.y = layout[id].position.y.round();

    layout[id].size.width = (abs_x + layout[id].size.width).round() - abs_x.round();
    layout[id].size.height = (abs_y + layout[id].size.height).round() - abs_y.round();

    for id in tree[id].child_ids() {
        round_layout(tree, layout, id, abs_x, abs_y);
    }
}

fn layout<T>(
    alloc: &Bump,
    tree: &[ArrayNode<T>],
    id: usize,
    bounds: Size,
    hypothetical: bool,
    cache: &mut [Cache],
    output: &mut [Layout],
) -> Size {
    //println!("id: {:?}, bounds: {:?}, hypo: {:?}", id, bounds, hypothetical);
    // Check cache for already calculated hypothetical outer size
    if let Some(result) = cache[id].inf_result {
        if hypothetical && bounds.is_infinite() {
            return result;
        }
    }

    if let Some(result) = cache[id].fin_result {
        // TODO - hopefully remove this
        let width_compat =
            (cache[id].bounds.width - bounds.width).abs() < f32::EPSILON || cache[id].bounds.width == bounds.width;
        let height_compat =
            (cache[id].bounds.height - bounds.height).abs() < f32::EPSILON || cache[id].bounds.height == bounds.height;

        if hypothetical && (!width_compat || !height_compat) {
            //println!("Bounds: {:?}\n Cache:{:?}", bounds, cache[id].bounds);
            //panic!("Not compatable??");
        }

        if hypothetical && !bounds.is_infinite() {
            return result;
        }
    }

    // Define some useful constants
    let dir = tree[id].style.flex_direction;
    let align_content = tree[id].style.align_content;
    let justify_content = tree[id].style.justify_content;
    let flex_wrap = tree[id].style.flex_wrap;

    let margin = tree[id].style.margin();
    let border = tree[id].style.border();
    let padding = tree[id].style.padding();
    let mbp = margin + border + padding;
    let min_size = Size::new(tree[id].style.min_width, tree[id].style.min_height);
    let max_size = Size::new(tree[id].style.max_width, tree[id].style.max_height);

    let mut container_size = Size::zero();

    // leaf nodes can skip the rest of the function
    if tree[id].num_children == 0 {
        // TODO - measure content

        let width = bounds.width.finite_or_else(|| {
            tree[id]
                .style
                .width
                .unwrap_or(0.0)
                .min(tree[id].style.max_width)
                .max(tree[id].style.min_width)
                + mbp.horizontal()
        });

        let height = bounds.height.finite_or_else(|| {
            tree[id]
                .style
                .height
                .unwrap_or(0.0)
                .min(tree[id].style.max_height)
                .max(tree[id].style.min_height)
                + mbp.vertical()
        });

        container_size = Size { width, height };

        if bounds.is_infinite() {
            cache[id].inf_result = Some(container_size);
        } else {
            cache[id].fin_result = Some(container_size);
            cache[id].bounds = bounds;
        }

        return container_size;
    }

    // 1 - Generate anonymous flex items
    let flex_items_iter = tree[id]
        .child_ids()
        .rev()
        .map(|id| (id, &tree[id].style))
        .filter(|(_, style)| style.position != Position::Fixed)
        .map(|(id, style)| FlexItem {
            id,

            min_size: Size::new(style.min_width, style.min_height),
            max_size: Size::new(style.max_width, style.max_height),

            position: style.position(),
            margin: style.margin(),
            mbp: style.margin() + style.border() + style.padding(),

            flex_basis: 0.0,
            flex_grow: style.flex_grow,
            flex_shrink: style.flex_shrink,

            violation: 0.0,
            frozen: false,

            hypo_outer_size: Size::zero(),
            hypo_inner_size: Size::zero(),
            target_size: Size::zero(),

            baseline: 0.0,

            offset_main: 0.0,
            offset_cross: 0.0,
        });
    let mut flex_items = BumpVec::from_iter_in(flex_items_iter, alloc);

    let has_baseline_child = flex_items
        .iter()
        .any(|item| tree[item.id].style.align_self == AlignItems::Baseline);

    // 2 - Determine the available main and cross space for the flex items
    let available_space = Size {
        width: bounds.width - mbp.horizontal(),
        height: bounds.height - mbp.vertical(),
    };

    // 3 - Determine the flex base size and hypothetical main size of each item
    for item in &mut flex_items {
        let inf_result = layout(alloc, tree, item.id, Size::infinite(), true, cache, output);

        // A - If the item has a definite used flex basis, that’s the flex base size
        if let Some(flex_basis) = tree[item.id].style.flex_basis {
            item.flex_basis = flex_basis;
        } else {
            item.flex_basis = inf_result.main(dir) - item.mbp.main(dir);
        };

        item.hypo_inner_size = (inf_result - item.mbp.size()).min(item.max_size).max(item.min_size);
        item.hypo_outer_size = item.hypo_inner_size + item.mbp.size();
    }

    // 5 - Collect flex items into flex lines
    let mut flex_lines: BumpVec<FlexLine> = BumpVec::new_in(alloc);

    if tree[id].style.flex_wrap == FlexWrap::NoWrap {
        flex_lines.push(FlexLine {
            items: &mut flex_items,
            cross_size: 0.0,
            offset_cross: 0.0,
        });
    } else {
        let mut flex_items = &mut flex_items[..];

        while !flex_items.is_empty() {
            let mut line_length = 0.0;
            let index = flex_items
                .iter()
                .enumerate()
                .find(|&(i, item)| {
                    line_length += item.hypo_outer_size.main(dir);
                    line_length > bounds.main(dir) && i != 0
                })
                .map(|(i, _)| i)
                .unwrap_or(flex_items.len());

            let (items, rest) = flex_items.split_at_mut(index);
            flex_lines.push(FlexLine {
                items,
                cross_size: 0.0,
                offset_cross: 0.0,
            });
            flex_items = rest;
        }
    };

    // 6 - Determine main size of items
    for line in &mut flex_lines {
        // 9.7.1 - Determine used flex factor
        let total_hypo_outer_size: f32 = line.items.iter().map(|item| item.hypo_outer_size.main(dir)).sum();
        let growing: bool = total_hypo_outer_size < available_space.main(dir);

        // 9.7.2 - Size inflexible items
        for item in line.items.iter_mut() {
            if tree[item.id].style.flex_grow == 0.0
                || (growing && item.flex_basis > item.hypo_inner_size.main(dir))
                || (!growing && item.flex_basis < item.hypo_inner_size.main(dir))
            {
                item.target_size.set_main(dir, item.hypo_inner_size.main(dir));
                item.frozen = true;
            }
        }

        // 9.7.3 - Calculate initial free space
        let initial_free_space: f32 = available_space.main(dir) - total_hypo_outer_size;

        // 9.7.4 - Loop
        loop {
            // a. Check for flexible items
            if line.items.iter().all(|item| item.frozen) {
                break;
            }

            // b. Calculate the remaining free space
            let used_space: f32 = line.items.iter().map(|item| if item.frozen { item.target_size.main(dir) } else { item.flex_basis } + item.mbp.main(dir)).sum();

            let mut unfrozen = BumpVec::from_iter_in(line.items.iter_mut().filter(|item| !item.frozen), alloc);

            let (sum_flex_grow, sum_flex_shrink): (f32, f32) =
                unfrozen.iter().fold((0.0, 0.0), |(flex_grow, flex_shrink), item| {
                    (flex_grow + item.flex_grow, flex_shrink + item.flex_shrink)
                });

            let free_space = if growing && sum_flex_grow < 1.0 {
                (initial_free_space * sum_flex_grow).min(available_space.main(dir) - used_space)
            } else if !growing && sum_flex_shrink < 1.0 {
                (initial_free_space * sum_flex_shrink).max(available_space.main(dir) - used_space)
            } else {
                (available_space.main(dir) - used_space).max(0.0)
            };

            // c. Distribute the free space proportional the the flex factors
            if free_space.is_normal() {
                if growing && sum_flex_grow > 0.0 {
                    for item in &mut unfrozen {
                        item.target_size
                            .set_main(dir, item.flex_basis + free_space * (item.flex_grow / sum_flex_grow));
                    }
                } else if !growing && sum_flex_shrink > 0.0 {
                    let sum_scaled_shrink_factor: f32 = unfrozen.iter().map(|item| item.flex_basis * item.flex_shrink).sum();

                    if sum_scaled_shrink_factor > 0.0 {
                        for item in &mut unfrozen {
                            let scaled_shrink_factor = item.flex_basis * item.flex_shrink;
                            item.target_size.set_main(
                                dir,
                                item.flex_basis - free_space.abs() * (scaled_shrink_factor / sum_scaled_shrink_factor),
                            )
                        }
                    }
                }
            }

            // d. Fix min/max violations
            let total_violation = unfrozen.iter_mut().fold(0.0, |acc, item| {
                let prev_target = item.target_size.main(dir);
                item.target_size.set_main(
                    dir,
                    prev_target.min(item.max_size.main(dir)).max(0.0).max(item.min_size.main(dir)),
                );
                item.violation = item.target_size.main(dir) - prev_target;

                acc + item.violation
            });

            // e. Freeze over-flexed items
            for item in &mut unfrozen {
                match total_violation {
                    v if v > 0.0 => item.frozen = item.violation > 0.0,
                    v if v < 0.0 => item.frozen = item.violation < 0.0,
                    _ => item.frozen = true,
                }
            }
        }
    }

    // 7 - Determine the hypothetical cross size of each item
    for line in &mut flex_lines {
        for item in line.items.iter_mut() {
            let mut item_bounds = Size::infinite();
            item_bounds.set_main(dir, item.target_size.main(dir));
            let fin_result = layout(alloc, tree, item.id, item_bounds, true, cache, output);

            item.hypo_outer_size.set_cross(dir, fin_result.cross(dir));
            item.hypo_inner_size
                .set_cross(dir, fin_result.cross(dir) - item.mbp.cross(dir));
        }
    }

    // 8 - Calculate the cross size of each flex line
    // TODO - handle baselines
    if flex_lines.len() == 1 && available_space.cross(dir).is_finite() {
        flex_lines[0].cross_size = available_space.cross(dir);
    } else {
        for line in &mut flex_lines {
            line.cross_size = line
                .items
                .iter()
                .map(|item| item.hypo_outer_size.cross(dir))
                .fold(0.0, |acc, rhs| acc.max(rhs));
        }
    }

    // 9 - Handle 'align-content: stretch'
    if align_content == AlignContent::Stretch && available_space.cross(dir).is_finite() {
        let total_cross: f32 = flex_lines.iter().map(|line| line.cross_size).sum();
        let inner_cross = available_space.cross(dir);

        if total_cross < inner_cross {
            let remaining = inner_cross - total_cross;
            let additional = remaining / flex_lines.len() as f32;
            flex_lines.iter_mut().for_each(|line| line.cross_size += additional);
        }
    }

    // 11 - Determine the used cross size of each flex item
    for line in &mut flex_lines {
        for item in line.items.iter_mut() {
            let item_style = &tree[item.id].style;
            if item_style.align_self == AlignItems::Stretch
                && item_style.cross_margin_start(dir).is_some()
                && item_style.cross_margin_end(dir).is_some()
                && item_style.cross_size(dir).is_none()
            {
                item.target_size.set_cross(
                    dir,
                    (line.cross_size - item.mbp.cross(dir))
                        .min(item.max_size.cross(dir))
                        .max(item.min_size.cross(dir)),
                );
            } else {
                item.target_size.set_cross(dir, item.hypo_inner_size.cross(dir));
            }
        }
    }

    // 12 - Main-Axis Alignment: Distribute any remaining free space
    for line in &mut flex_lines {
        let used_space: f32 = line
            .items
            .iter()
            .map(|item| item.target_size.main(dir) + item.mbp.main(dir))
            .sum();
        let free_space = available_space.main(dir) - used_space;
        let mut num_auto_margins = 0;

        for item in line.items.iter_mut() {
            let item_style = &tree[item.id].style;
            if item_style.main_margin_start(dir).is_none() {
                num_auto_margins += 1;
            }
            if item_style.main_margin_end(dir).is_none() {
                num_auto_margins += 1;
            }
        }

        if free_space > 0.0 && num_auto_margins > 0 {
            let margin = free_space / num_auto_margins as f32;

            for item in line.items.iter_mut() {
                let item_style = &tree[item.id].style;
                if item_style.main_margin_start(dir).is_none() {
                    if dir.is_row() {
                        item.margin.left = margin;
                    } else {
                        item.margin.top = margin;
                    }
                }
                if item_style.main_margin_end(dir).is_none() {
                    if dir.is_row() {
                        item.margin.right = margin;
                    } else {
                        item.margin.bottom = margin;
                    }
                }
            }
        } else {
            let num_items = line.items.len();

            let justify_item = |(i, item): (usize, &mut FlexItem)| {
                let is_first = i == 0;

                item.offset_main = match justify_content {
                    JustifyContent::FlexStart => {
                        if is_first && dir.is_reverse() {
                            free_space
                        } else {
                            0.0
                        }
                    }
                    JustifyContent::FlexEnd => {
                        if is_first && !dir.is_reverse() {
                            free_space
                        } else {
                            0.0
                        }
                    }
                    JustifyContent::Center => {
                        if is_first {
                            free_space / 2.0
                        } else {
                            0.0
                        }
                    }
                    JustifyContent::SpaceBetween => {
                        if is_first {
                            0.0
                        } else {
                            free_space / (num_items - 1) as f32
                        }
                    }
                    JustifyContent::SpaceAround => {
                        if is_first {
                            (free_space / num_items as f32) / 2.0
                        } else {
                            free_space / num_items as f32
                        }
                    }
                    JustifyContent::SpaceEvenly => free_space / (num_items + 1) as f32,
                };
            };

            if dir.is_reverse() {
                line.items.iter_mut().rev().enumerate().for_each(justify_item);
            } else {
                line.items.iter_mut().enumerate().for_each(justify_item);
            }
        }
    }

    // 13 - Resolve cross-axis auto margins
    for line in &mut flex_lines {
        for item in line.items.iter_mut() {
            let item_style = &tree[item.id].style;
            let free_space = line.cross_size - item.target_size.cross(dir);

            if item_style.cross_margin_start(dir).is_none() && item_style.cross_margin_end(dir).is_none() {
                if dir.is_row() {
                    item.margin.top = free_space / 2.0;
                    item.margin.bottom = free_space / 2.0;
                } else {
                    item.margin.left = free_space / 2.0;
                    item.margin.right = free_space / 2.0;
                }
            } else if item_style.cross_margin_start(dir).is_none() {
                if dir.is_row() {
                    item.margin.top = free_space;
                } else {
                    item.margin.left = free_space;
                }
            } else if item_style.cross_margin_end(dir).is_none() {
                if dir.is_row() {
                    item.margin.bottom = free_space;
                } else {
                    item.margin.right = free_space;
                }
            } else {
                // 14 - Align all flex items along the cross-axis per align-self
                item.offset_cross = match item_style.align_self {
                    AlignItems::Stretch => {
                        if dir.is_reverse() {
                            free_space
                        } else {
                            0.0
                        }
                    }
                    AlignItems::Center => free_space / 2.0,
                    AlignItems::FlexStart => {
                        if dir.is_reverse() {
                            free_space
                        } else {
                            0.0
                        }
                    }
                    AlignItems::FlexEnd => {
                        if dir.is_reverse() {
                            0.0
                        } else {
                            free_space
                        }
                    }
                    AlignItems::Baseline => {
                        0.0 // TODO
                    }
                };
            }
        }
    }

    // 15 - Determine the flex container’s used cross size
    let total_cross_size: f32 = flex_lines.iter().map(|line| line.cross_size).sum();

    let inner_cross = if let Some(cross_size) = tree[id].style.cross_size(dir) {
        cross_size
    } else {
        total_cross_size
    }
    .min(max_size.cross(dir))
    .max(min_size.cross(dir));

    container_size.set_cross(dir, bounds.cross(dir).finite_or_else(|| inner_cross + mbp.cross(dir)));

    if hypothetical {
        if bounds.is_infinite() {
            cache[id].inf_result = Some(container_size);
        } else {
            cache[id].fin_result = Some(container_size);
            cache[id].bounds = bounds;
        }
        return container_size;
    }

    // 16 - Align all flex lines per align-content
    let free_space = inner_cross - total_cross_size;
    let num_lines = flex_lines.len();

    let align_line = |(i, line): (usize, &mut FlexLine)| {
        let is_first = i == 0;

        line.offset_cross = match align_content {
            AlignContent::Center => {
                if is_first {
                    free_space / 2.0
                } else {
                    0.0
                }
            }
            AlignContent::FlexEnd => {
                if is_first && !dir.is_reverse() {
                    free_space
                } else {
                    0.0
                }
            }
            AlignContent::FlexStart => {
                if is_first && dir.is_reverse() {
                    free_space
                } else {
                    0.0
                }
            }
            AlignContent::SpaceAround => {
                if is_first {
                    (free_space / num_lines as f32) / 2.0
                } else {
                    free_space / num_lines as f32
                }
            }
            AlignContent::SpaceBetween => {
                if is_first {
                    0.0
                } else {
                    free_space / (num_lines - 1) as f32
                }
            }
            AlignContent::Stretch => 0.0,
        };
    };

    if dir.is_reverse() {
        flex_lines.iter_mut().rev().enumerate().for_each(align_line);
    } else {
        flex_lines.iter_mut().enumerate().for_each(align_line);
    }

    // Save final layouts
    let mut total_offset_cross = border.cross_start(dir) + padding.cross_start(dir);
    let layout_line = |line: &mut FlexLine| {
        let mut total_offset_main =
            tree[id].style.main_margin_start(dir).unwrap_or(0.0) + border.main_start(dir) + padding.main_start(dir);
        let line_offset_cross = line.offset_cross;

        // TODO - support CSS position
        let layout_item = |item: &mut FlexItem| {
            // Now that we know the final size of an item, layout its children
            layout(alloc, tree, item.id, item.target_size, false, cache, output);

            let offset_main = total_offset_main + item.offset_main + item.margin.main_start(dir);
            let offset_cross = total_offset_cross + item.offset_cross + line_offset_cross + item.margin.cross_start(dir);

            output[item.id] = Layout {
                size: item.target_size,
                position: Point {
                    x: if dir.is_row() { offset_main } else { offset_cross },
                    y: if !dir.is_row() { offset_main } else { offset_cross },
                },
            };

            total_offset_main += item.offset_main + item.mbp.main(dir) + item.target_size.main(dir);
        };

        if dir.is_reverse() {
            line.items.iter_mut().rev().for_each(layout_item);
        } else {
            line.items.iter_mut().for_each(layout_item);
        }

        total_offset_cross += line_offset_cross + line.cross_size;
    };

    if flex_wrap == FlexWrap::WrapReverse {
        flex_lines.iter_mut().rev().for_each(layout_line);
    } else {
        flex_lines.iter_mut().for_each(layout_line);
    }

    container_size
}

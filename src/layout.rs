#![forbid(unsafe_code)]

use crate::geometry::*;
use crate::style::*;
use crate::tree::ArrayNode;

use bumpalo::{collections::Vec as BumpVec, Bump};

impl Style {
    fn size(&self) -> Size {
        Size::new(self.width.unwrap_or(0.0), self.height.unwrap_or(0.0))
    }

    fn min_size(&self) -> Size {
        Size::new(self.min_width, self.min_height)
    }

    fn max_size(&self) -> Size {
        Size::new(self.max_width, self.max_height)
    }

    fn position(&self) -> Rect {
        Rect::new(
            self.top.unwrap_or(0.0),
            self.right.unwrap_or(0.0),
            self.bottom.unwrap_or(0.0),
            self.left.unwrap_or(0.0),
        )
    }

    fn margin(&self) -> Rect {
        Rect::new(
            self.margin_top.unwrap_or(0.0),
            self.margin_right.unwrap_or(0.0),
            self.margin_bottom.unwrap_or(0.0),
            self.margin_left.unwrap_or(0.0),
        )
    }

    fn border(&self) -> Rect {
        Rect::new(
            self.border_top_width,
            self.border_right_width,
            self.border_bottom_width,
            self.border_left_width,
        )
    }

    fn padding(&self) -> Rect {
        Rect::new(self.padding_top, self.padding_right, self.padding_bottom, self.padding_left)
    }
}

#[derive(Debug)]
struct FlexItem {
    id: usize,

    align_self: AlignItems,

    min_size: Size,
    max_size: Size,

    position: Rect,
    margin: Rect,
    border_padding: Rect,

    auto_main_start: bool,
    auto_main_end: bool,
    auto_cross_start: bool,
    auto_cross_end: bool,
    auto_cross_size: bool,

    flex_basis: f32,
    flex_grow: f32,
    flex_shrink: f32,

    hypo_inner_size: Size,
    hypo_outer_size: Size,

    violation: f32,
    frozen: bool,

    offset_main: f32,
    offset_cross: f32,
    target_size: Size,
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

pub(crate) fn build_layout<T>(temp: &Bump, tree: &[ArrayNode<T>], root_size: Size, output: &mut [Layout]) {
    layout(temp, tree, 0, root_size, output);
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

fn layout<T>(temp: &Bump, tree: &[ArrayNode<T>], id: usize, size: Size, output: &mut [Layout]) {
    // leaf nodes don't need to do anything
    if tree[id].num_children == 0 {
        return;
    }

    // Define some useful constants
    let dir = tree[id].style.flex_direction;
    let align_content = tree[id].style.align_content;
    let justify_content = tree[id].style.justify_content;
    let flex_wrap = tree[id].style.flex_wrap;
    let border_padding = tree[id].style.border() + tree[id].style.padding();

    // 1 - Generate anonymous flex items
    let flex_items_iter = tree[id]
        .child_ids()
        .rev()
        .map(|id| (id, &tree[id].style))
        .filter(|(_, style)| style.position != Position::Fixed)
        .map(|(id, style)| {
            let min_size = style.min_size();
            let max_size = style.max_size();
            let border_padding = style.border() + style.padding();
            let flex_basis = style.flex_basis.unwrap_or_else(|| style.size().main(dir));
            let hypo_inner_size = style.size().with_main(dir, flex_basis).clamp(min_size, max_size);
            let hypo_outer_size = hypo_inner_size + border_padding.size();

            FlexItem {
                id,

                align_self: style.align_self,

                min_size,
                max_size,

                position: style.position(),
                margin: style.margin(),
                border_padding,

                auto_main_start: if dir.is_row() { style.margin_left } else { style.margin_top }.is_none(),
                auto_main_end: if dir.is_row() { style.margin_right } else { style.margin_bottom }.is_none(),
                auto_cross_start: if !dir.is_row() { style.margin_left } else { style.margin_top }.is_none(),
                auto_cross_end: if !dir.is_row() { style.margin_right } else { style.margin_bottom }.is_none(),
                auto_cross_size: if !dir.is_row() { style.width } else { style.height }.is_none(),

                flex_basis,
                flex_grow: style.flex_grow,
                flex_shrink: style.flex_shrink,

                hypo_inner_size,
                hypo_outer_size,

                violation: 0.0,
                frozen: false,

                offset_main: 0.0,
                offset_cross: 0.0,
                target_size: Size::zero(),
            }
        });
    let mut flex_items = BumpVec::from_iter_in(flex_items_iter, temp);

    // 5 - Collect flex items into flex lines
    let mut flex_lines: BumpVec<FlexLine> = BumpVec::new_in(temp);

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
                    line_length > size.main(dir) && i != 0
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
        let total_hypo_outer_size: f32 = line
            .items
            .iter()
            .map(|item| item.hypo_outer_size.main(dir) + item.margin.main(dir))
            .sum();
        let growing: bool = total_hypo_outer_size < size.main(dir);

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
        let initial_free_space: f32 = size.main(dir) - total_hypo_outer_size;

        // 9.7.4 - Loop
        loop {
            // a. Check for flexible items
            if line.items.iter().all(|item| item.frozen) {
                break;
            }

            // b. Calculate the remaining free space
            let used_space: f32 = line
                .items
                .iter()
                .map(|item| {
                    (if item.frozen { item.target_size.main(dir) } else { item.flex_basis })
                        + item.margin.main(dir)
                        + item.border_padding.main(dir)
                })
                .sum();

            let mut unfrozen = BumpVec::from_iter_in(line.items.iter_mut().filter(|item| !item.frozen), temp);

            let (sum_flex_grow, sum_flex_shrink): (f32, f32) = unfrozen.iter().fold((0.0, 0.0), |(flex_grow, flex_shrink), item| {
                (flex_grow + item.flex_grow, flex_shrink + item.flex_shrink)
            });

            let free_space = if growing && sum_flex_grow < 1.0 {
                (initial_free_space * sum_flex_grow).min(size.main(dir) - used_space)
            } else if !growing && sum_flex_shrink < 1.0 {
                (initial_free_space * sum_flex_shrink).max(size.main(dir) - used_space)
            } else {
                size.main(dir) - used_space
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
                                item.flex_basis + free_space * (scaled_shrink_factor / sum_scaled_shrink_factor),
                            )
                        }
                    }
                }
            }

            // d. Fix min/max violations
            let total_violation = unfrozen.iter_mut().fold(0.0, |acc, item| {
                let prev_target = item.target_size.main(dir);
                item.target_size
                    .set_main(dir, prev_target.min(item.max_size.main(dir)).max(item.min_size.main(dir)));
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

    // 8 - Calculate the cross size of each flex line
    if flex_lines.len() == 1 {
        flex_lines[0].cross_size = size.cross(dir);
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
    if align_content == AlignContent::Stretch {
        let total_cross: f32 = flex_lines.iter().map(|line| line.cross_size).sum();
        let inner_cross = size.cross(dir);

        if total_cross < inner_cross {
            let remaining = inner_cross - total_cross;
            let additional = remaining / flex_lines.len() as f32;
            flex_lines.iter_mut().for_each(|line| line.cross_size += additional);
        }
    }

    // 11 - Determine the used cross size of each flex item
    for line in &mut flex_lines {
        for item in line.items.iter_mut() {
            if item.align_self == AlignItems::Stretch && !item.auto_cross_start && !item.auto_cross_end && item.auto_cross_size {
                item.target_size.set_cross(
                    dir,
                    (line.cross_size - item.margin.cross(dir) - item.border_padding.cross(dir))
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
            .map(|item| item.target_size.main(dir) + item.border_padding.main(dir))
            .sum();
        let free_space = size.main(dir) - used_space;
        let mut num_auto_margins = 0;

        for item in line.items.iter_mut() {
            if item.auto_main_start {
                num_auto_margins += 1;
            }
            if item.auto_main_end {
                num_auto_margins += 1;
            }
        }

        if free_space > 0.0 && num_auto_margins > 0 {
            let margin = free_space / num_auto_margins as f32;

            for item in line.items.iter_mut() {
                if item.auto_main_start {
                    if dir.is_row() {
                        item.margin.left = margin;
                    } else {
                        item.margin.top = margin;
                    }
                }
                if item.auto_main_end {
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
            let free_space = line.cross_size - item.target_size.cross(dir);

            if item.auto_cross_start && item.auto_cross_end {
                if dir.is_row() {
                    item.margin.top = free_space / 2.0;
                    item.margin.bottom = free_space / 2.0;
                } else {
                    item.margin.left = free_space / 2.0;
                    item.margin.right = free_space / 2.0;
                }
            } else if item.auto_cross_start {
                if dir.is_row() {
                    item.margin.top = free_space;
                } else {
                    item.margin.left = free_space;
                }
            } else if item.auto_cross_end {
                if dir.is_row() {
                    item.margin.bottom = free_space;
                } else {
                    item.margin.right = free_space;
                }
            } else {
                // 14 - Align all flex items along the cross-axis per align-self
                item.offset_cross = match item.align_self {
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
                };
            }
        }
    }

    // 16 - Align all flex lines per align-content
    let free_space = size.cross(dir);
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
    let mut total_offset_cross = border_padding.cross_start(dir);
    let layout_line = |line: &mut FlexLine| {
        let mut total_offset_main = border_padding.main_start(dir);
        let line_offset_cross = line.offset_cross;

        // TODO - support CSS position
        let layout_item = |item: &mut FlexItem| {
            // Now that we know the final size of an item, layout its children
            layout(temp, tree, item.id, item.target_size, output);

            let offset_main = total_offset_main + item.offset_main + item.margin.main_start(dir);
            let offset_cross = total_offset_cross + item.offset_cross + line_offset_cross + item.margin.cross_start(dir);

            output[item.id] = Layout {
                size: item.target_size + item.border_padding.size(),
                position: Point {
                    x: if dir.is_row() { offset_main } else { offset_cross },
                    y: if !dir.is_row() { offset_main } else { offset_cross },
                },
            };

            total_offset_main += item.offset_main + item.target_size.main(dir) + item.border_padding.main(dir) + item.margin.main(dir);
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
}

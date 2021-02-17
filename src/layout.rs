use crate::geometry::*;
use crate::minmax::*;
use crate::style::*;
use crate::tree::ArrayNode;

impl Style {
    fn flex_base(&self, dir: FlexDirection) -> f32 {
        if dir.is_row() {
            self.flex_basis.unwrap_or_else(|| self.width.unwrap_or(0.0))
        } else {
            self.flex_basis.unwrap_or_else(|| self.height.unwrap_or(0.0))
        }
    }

    fn hypo_inner_size(&self, dir: FlexDirection) -> Size {
        if dir.is_row() {
            Size {
                width: self.min_width.maybe_max(
                    self.max_width
                        .maybe_min(self.flex_basis.unwrap_or_else(|| self.width.unwrap_or(0.0))),
                ),
                height: self
                    .min_height
                    .maybe_max(self.max_height.maybe_min(self.height.unwrap_or(0.0))),
            }
        } else {
            Size {
                width: self
                    .min_width
                    .maybe_max(self.max_width.maybe_min(self.width.unwrap_or(0.0))),
                height: self.min_height.maybe_max(
                    self.max_height
                        .maybe_min(self.flex_basis.unwrap_or_else(|| self.height.unwrap_or(0.0))),
                ),
            }
        }
    }

    fn perimeter_size(&self, dir: FlexDirection) -> Size {
        Size {
            width: self.margin_left.unwrap_or(0.0)
                + self.margin_right.unwrap_or(0.0)
                + self.border_left_width
                + self.border_right_width
                + self.padding_left
                + self.padding_right,
            height: self.margin_top.unwrap_or(0.0)
                + self.margin_bottom.unwrap_or(0.0)
                + self.border_top_width
                + self.border_bottom_width
                + self.padding_top
                + self.padding_bottom,
        }
    }

    fn hypo_outer_size(&self, dir: FlexDirection) -> Size {
        let hypo_inner = self.hypo_inner_size(dir);
        let perimeter = self.perimeter_size(dir);

        Size {
            width: hypo_inner.width + perimeter.width,
            height: hypo_inner.height + perimeter.height,
        }
    }
    /*
    fn initial_target_size(&self, dir: FlexDirection) -> Size {
        if dir.is_row() {
            Size {
                width: self.min_width.maybe_max(
                    self.max_width
                        .maybe_min(self.flex_basis.unwrap_or_else(|| self.width.unwrap_or(0.0))),
                ),
                height: self
                    .min_height
                    .maybe_max(self.max_height.maybe_min(self.height.unwrap_or(0.0))),
            }
        } else {
            Size {
                width: self
                    .min_width
                    .maybe_max(self.max_width.maybe_min(self.width.unwrap_or(0.0))),
                height: self.min_height.maybe_max(
                    self.max_height
                        .maybe_min(self.flex_basis.unwrap_or_else(|| self.height.unwrap_or(0.0))),
                ),
            }
        }
    }

    fn hypothetical_inner_size(&self, dir: FlexDirection) -> Size {
        if dir.is_row() {
            Size {
                width: self.min_width.maybe_max(
                    self.max_width
                        .maybe_min(self.flex_basis.unwrap_or_else(|| self.width.unwrap_or(0.0))),
                ) + self.border_left_width
                    + self.border_right_width
                    + self.padding_left
                    + self.padding_right,
                height: self
                    .min_height
                    .maybe_max(self.max_height.maybe_min(self.height.unwrap_or(0.0)))
                    + self.border_top_width
                    + self.border_bottom_width
                    + self.padding_top
                    + self.padding_bottom,
            }
        } else {
            Size {
                width: self
                    .min_width
                    .maybe_max(self.max_width.maybe_min(self.width.unwrap_or(0.0)))
                    + self.border_left_width
                    + self.border_right_width
                    + self.padding_left
                    + self.padding_right,
                height: self.min_height.maybe_max(
                    self.max_height
                        .maybe_min(self.flex_basis.unwrap_or_else(|| self.height.unwrap_or(0.0))),
                ) + self.border_top_width
                    + self.border_bottom_width
                    + self.padding_top
                    + self.padding_bottom,
            }
        }
    }

    fn hypothetical_outer_size(&self, dir: FlexDirection) -> Size {
        if dir.is_row() {
            Size {
                width: self.min_width.maybe_max(
                    self.max_width
                        .maybe_min(self.flex_basis.unwrap_or_else(|| self.width.unwrap_or(0.0))),
                ) + self.margin_left.unwrap_or(0.0)
                    + self.margin_right.unwrap_or(0.0)
                    + self.border_left_width
                    + self.border_right_width
                    + self.padding_left
                    + self.padding_right,
                height: self
                    .min_height
                    .maybe_max(self.max_height.maybe_min(self.height.unwrap_or(0.0)))
                    + self.margin_top.unwrap_or(0.0)
                    + self.margin_bottom.unwrap_or(0.0)
                    + self.border_top_width
                    + self.border_bottom_width
                    + self.padding_top
                    + self.padding_bottom,
            }
        } else {
            Size {
                width: self
                    .min_width
                    .maybe_max(self.max_width.maybe_min(self.width.unwrap_or(0.0)))
                    + self.margin_left.unwrap_or(0.0)
                    + self.margin_right.unwrap_or(0.0)
                    + self.border_left_width
                    + self.border_right_width
                    + self.padding_left
                    + self.padding_right,
                height: self.min_height.maybe_max(
                    self.max_height
                        .maybe_min(self.flex_basis.unwrap_or_else(|| self.height.unwrap_or(0.0))),
                ) + self.margin_top.unwrap_or(0.0)
                    + self.margin_bottom.unwrap_or(0.0)
                    + self.border_top_width
                    + self.border_bottom_width
                    + self.padding_top
                    + self.padding_bottom,
            }
        }
    }*/
}

#[derive(Debug)]
struct FlexItem {
    id: usize,

    margin_top: Option<f32>,
    margin_right: Option<f32>,
    margin_bottom: Option<f32>,
    margin_left: Option<f32>,

    align_self: AlignItems,
    flex_grow: f32,
    flex_shrink: f32,
    flex_base: f32,

    min_size: Size,
    max_size: Size,

    hypo_inner_size: Size,
    hypo_outer_size: Size,
    perimeter_size: Size,

    target_size: Size,
    frozen: bool,
}

impl FlexItem {
    fn flexed_size(&self, flex_space: f32, total_flex_factor: f32) -> f32 {
        (flex_space * (self.flex_grow / total_flex_factor)) + self.flex_base
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct Layout {
    pub size: Size,
    pub position: Point,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            size: Size {
                width: 0.0,
                height: 0.0,
            },
            position: Point { x: 0.0, y: 0.0 },
        }
    }
}

// TODO content should be drawn behind children
#[allow(dead_code)]
impl Layout {
    pub fn hit_test<T>(tree: &[ArrayNode<T>], layout: &[Self], position: (f32, f32)) -> usize {
        Self::hit_test_node(tree, layout, position, (0.0, 0.0), 0)
    }

    fn hit_test_node<T>(
        tree: &[ArrayNode<T>],
        layout: &[Self],
        position: (f32, f32),
        offset: (f32, f32),
        node: usize,
    ) -> usize {
        let mut hit_node = node;
        let child_ids: Vec<usize> = if let Some(last_child) = tree[node].last_child {
            (last_child.get()..last_child.get() + tree[node].num_children)
                .rev()
                .collect()
        } else {
            Vec::new()
        };

        // TODO in order of z-depth
        for id in child_ids {
            if position.0 >= offset.0 + layout[id].position.x
                && position.1 >= offset.1 + layout[id].position.y
                && position.0 <= offset.0 + layout[id].position.x + layout[id].size.width
                && position.1 <= offset.1 + layout[id].position.y + layout[id].size.height
            {
                hit_node = Self::hit_test_node(
                    tree,
                    layout,
                    position,
                    (offset.0 + layout[id].position.x, offset.1 + layout[id].position.y),
                    id,
                );
            }
        }

        hit_node
    }

    fn round_layout<T>(tree: &[ArrayNode<T>], layouts: &mut [Self], id: usize, abs_x: f32, abs_y: f32) {
        let abs_x = abs_x + layouts[id].position.x;
        let abs_y = abs_y + layouts[id].position.y;

        layouts[id].position.x = layouts[id].position.x.round();
        layouts[id].position.y = layouts[id].position.y.round();

        layouts[id].size.width = (abs_x + layouts[id].size.width).round() - abs_x.round();
        layouts[id].size.height = (abs_y + layouts[id].size.height).round() - abs_y.round();

        for id in tree[id].child_ids() {
            Self::round_layout(tree, layouts, id, abs_x, abs_y);
        }
    }

    pub fn solve<T>(tree: &[ArrayNode<T>], size: (f32, f32), output: &mut [Self]) {
        let bounds = Bounds {
            min_width: size.0 as f32,
            max_width: size.0 as f32,
            min_height: size.1 as f32,
            max_height: size.1 as f32,
        };

        /*output[0] = Self {
            size: Self::solve_node(0, tree, bounds, output),
            ..Self::default()
        };*/

        output[0] = Self {
            size: Size {
                width: size.0,
                height: size.1,
            },
            ..Self::default()
        };

        Self::round_layout(tree, output, 0, 0.0, 0.0);
    }

    // Returns size including borders and padding, but not margins
    fn solve_node<T>(id: usize, tree: &[ArrayNode<T>], bounds: Bounds, layouts: &mut [Self]) -> Size {
        let container = &tree[id];
        let dir = container.style.flex_direction;

        // Collect children that aren't position: fixed
        // TODO make sure to deal with position: fixed children at the end
        let mut flex_items: Vec<FlexItem> = container
            .child_ids()
            .rev()
            .map(|id| (id, &tree[id].style))
            .filter(|(_, style)| style.position != Position::Fixed)
            .map(|(id, style)| FlexItem {
                id,
                margin_top: style.margin_top,
                margin_right: style.margin_right,
                margin_bottom: style.margin_bottom,
                margin_left: style.margin_left,
                align_self: style.align_self,
                flex_grow: style.flex_grow,
                flex_shrink: style.flex_shrink,
                flex_base: style.flex_base(dir),
                min_size: Size::new(style.min_width.unwrap_or(0.0), style.min_height.unwrap_or(0.0)),
                max_size: Size::new(
                    style.max_width.unwrap_or(f32::INFINITY),
                    style.max_height.unwrap_or(f32::INFINITY),
                ),
                hypo_inner_size: style.hypo_inner_size(dir),
                hypo_outer_size: style.hypo_outer_size(dir),
                perimeter_size: style.perimeter_size(dir),
                target_size: style.hypo_inner_size(dir),
                frozen: false,
            })
            .collect();

        // TODO layout text and stuff

        if flex_items.is_empty() {
            // TODO this is incorrect
            Size {
                width: bounds.min_width.max(bounds.max_width),
                height: bounds.min_height.max(bounds.max_height),
            }
        } else {
            // Split flex items into lines
            let mut lines: Vec<Vec<FlexItem>> = match container.style.flex_wrap {
                FlexWrap::NoWrap => vec![flex_items],
                _ => {
                    let mut lines = Vec::new();
                    while !flex_items.is_empty() {
                        let mut remaining_space = bounds.min_main(dir);
                        let mut line = Vec::new();
                        while remaining_space >= 0.0 && !flex_items.is_empty() {
                            if remaining_space - flex_items.last().unwrap().target_size.main(dir) >= 0.0 {
                                let item = flex_items.pop().unwrap();
                                remaining_space -= item.target_size.main(dir);
                                line.push(item);
                            }
                        }
                        lines.push(line);
                    }
                    lines
                }
            };

            // Determine main size of items
            for line in &mut lines {
                // 9.7.1 - Determine used flex factor
                let total_hypo_size: f32 = line.iter().map(|child| child.target_size.main(dir)).sum();
                let growing: bool = total_hypo_size < bounds.max_main(dir);

                // 9.7.2 - Size inflexible items
                for item in line.iter_mut() {
                    if item.flex_grow == 0.0
                        || (growing && item.flex_base > item.target_size.main(dir))
                        || (!growing && item.flex_base < item.target_size.main(dir))
                    {
                        item.frozen = true;
                    }
                }

                // 9.7.3 - Calculate initial free space
                let total_hypo_main_size: f32 = line.iter().map(|item| item.target_size.main(dir)).sum();
                let initial_free_space: f32 = bounds.max_main(dir) - total_hypo_main_size;

                // 9.7.4 - Loop
                loop {
                    // a. Check for flexible items
                    if line.iter().all(|item| item.frozen) {
                        break;
                    }

                    // b. Calculate the remaining free space
                    let used_space: f32 = line.iter().map(|item| item.target_size.main(dir)).sum();

                    let mut unfrozen: Vec<&mut FlexItem> = line.iter_mut().filter(|item| !item.frozen).collect();

                    let (sum_flex_grow, sum_flex_shrink): (f32, f32) =
                        unfrozen.iter().fold((0.0, 0.0), |(flex_grow, flex_shrink), item| {
                            (flex_grow + item.flex_grow, flex_shrink + item.flex_shrink)
                        });

                    let free_space = if growing && sum_flex_grow < 1.0 {
                        initial_free_space * sum_flex_grow
                    } else if !growing && sum_flex_shrink < 1.0 {
                        initial_free_space * sum_flex_shrink
                    } else {
                        bounds.max_main(dir) - used_space
                    };

                    // c. Distribute the free space proportional the the flex factors
                    if free_space.is_normal() {
                        if growing && sum_flex_grow > 0.0 {
                            for item in &mut unfrozen {
                                item.target_size
                                    .set_main(dir, item.flex_base + free_space * (item.flex_grow / sum_flex_grow));
                            }
                        } else if !growing && sum_flex_shrink > 0.0 {
                            let sum_scaled_shrink_factor: f32 =
                                unfrozen.iter().map(|item| item.flex_base * item.flex_shrink).sum();

                            if sum_scaled_shrink_factor > 0.0 {
                                for item in &mut unfrozen {
                                    let scaled_shrink_factor = item.flex_base * item.flex_shrink;
                                    item.target_size.set_main(
                                        dir,
                                        item.flex_base + free_space * (scaled_shrink_factor / sum_scaled_shrink_factor),
                                    )
                                }
                            }
                        }
                    }

                    // d. Fix min/max violations

                    // e. Freeze over-flexed items
                }
            }

            let mut x_pos: f32 = 0.0;
            for line in lines {
                // TODO: Position children

                for item in line {
                    let child_layout = Layout {
                        size: item.target_size,
                        position: Point { x: x_pos, y: 0.0 },
                    };
                    layouts[item.id] = child_layout;
                    x_pos += item.target_size.main(dir);
                }
            }

            // TODO: Return correct size, taking into account node's style as well as children
            Size {
                width: (x_pos).max(bounds.min_width).min(bounds.max_width),
                height: bounds.max_height,
            }
        }
    }
}

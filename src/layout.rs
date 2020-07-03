use std::{collections::HashMap, f32};

use crate::style::{FlexDirection, Point, Position, Size};
use crate::tree::ArrayNode;

#[derive(Debug, Default, Copy, Clone)]
pub struct Bounds {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
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

// TODO: warn when in debug mode if content is being ignored
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

    pub fn solve<T>(tree: &[ArrayNode<T>], size: (f64, f64)) -> Option<Vec<Self>> {
        let mut layouts = Vec::with_capacity(tree.len());
        for _ in 0..tree.len() {
            layouts.push(Layout::default());
        }

        let bounds = Bounds {
            min_width: size.0 as f32,
            max_width: size.0 as f32,
            min_height: size.1 as f32,
            max_height: size.1 as f32,
        };

        let mut root_layout = Self::default();
        root_layout.size = Self::solve_node(0, tree, bounds, &mut layouts)?;
        layouts[0] = root_layout;
        Self::round_layout(tree, &mut layouts, 0, 0.0, 0.0);
        Some(layouts)
    }

    // Returns size including borders and padding, but not margins
    fn solve_node<T>(id: usize, tree: &[ArrayNode<T>], bounds: Bounds, layouts: &mut [Self]) -> Option<Size> {
        debug_assert!(bounds.min_width <= bounds.max_width);
        debug_assert!(bounds.min_height <= bounds.max_height);

        // Collect children that aren't position: fixed
        let mut child_ids: Vec<usize> = tree[id]
            .child_ids()
            .filter(|child_id| tree[*child_id].style.position != Position::Fixed)
            .rev()
            .collect();

        if child_ids.is_empty() {
            // TODO layout text and stuff

            Some(Size {
                width: bounds.min_width.max(
                    bounds.max_width.min(
                        tree[id]
                            .style
                            .width
                            .unwrap_or_else(|| tree[id].style.min_width.unwrap_or(0.0))
                            + tree[id].style.border_left_width
                            + tree[id].style.border_right_width
                            + tree[id].style.padding_left
                            + tree[id].style.padding_right,
                    ),
                ),
                height: bounds.min_height.max(
                    bounds.max_height.min(
                        tree[id]
                            .style
                            .height
                            .unwrap_or_else(|| tree[id].style.min_height.unwrap_or(0.0))
                            + tree[id].style.border_top_width
                            + tree[id].style.border_bottom_width
                            + tree[id].style.padding_top
                            + tree[id].style.padding_bottom,
                    ),
                ),
            })
        } else {
            let mut child_sizes: HashMap<usize, Size> = HashMap::new();

            // Split children into fixed and flex groups
            let mut fixed_child_ids = Vec::new();
            let mut flex_child_ids = Vec::new();

            child_ids.sort_by_key(|&id| tree[id].style.order);

            for &child_id in child_ids.iter() {
                if tree[child_id].style.flex_grow == 0.0 {
                    fixed_child_ids.push(child_id);
                } else {
                    flex_child_ids.push(child_id);
                }
            }

            match tree[id].style.flex_direction {
                FlexDirection::RowReverse | FlexDirection::ColumnReverse => {
                    child_ids.reverse();
                }
                _ => {}
            }
            match tree[id].style.flex_direction {
                FlexDirection::Row | FlexDirection::RowReverse => {
                    // TODO: collect into multiple rows if flex-wrap

                    // Count how many auto margins there are
                    let mut num_auto_margins: u32 = 0;
                    for &child_id in child_ids.iter() {
                        if tree[child_id].style.margin_left == None {
                            num_auto_margins += 1;
                        }

                        if tree[child_id].style.margin_right == None {
                            num_auto_margins += 1;
                        }
                    }

                    // Get sizes of fixed children and calculate flexible space
                    let fixed_bounds = Bounds {
                        min_width: 0.0,
                        max_width: f32::INFINITY,
                        min_height: 0.0,
                        max_height: bounds.max_height,
                    };
                    let mut remaining_space = bounds.max_width;
                    for &id in fixed_child_ids.iter() {
                        let child_size = Self::solve_node(id, tree, fixed_bounds, layouts);
                        child_sizes.insert(id, child_size?);
                        remaining_space -= child_size?.width;
                    }

                    // Account for flex items' inflexible parts
                    for &id in flex_child_ids.iter() {
                        remaining_space -= tree[id]
                            .style
                            .flex_basis
                            .unwrap_or_else(|| tree[id].style.width.unwrap_or(0.0));

                        remaining_space -= tree[id].style.margin_left.unwrap_or(0.0);
                        remaining_space -= tree[id].style.margin_right.unwrap_or(0.0);

                        remaining_space -= tree[id].style.border_left_width;
                        remaining_space -= tree[id].style.border_right_width;

                        remaining_space -= tree[id].style.padding_left;
                        remaining_space -= tree[id].style.padding_right;
                    }

                    // Helper function
                    fn desired_width<T>(node: &ArrayNode<T>, flex_space: f32, total_flex_factor: f32) -> f32 {
                        (flex_space * (node.style.flex_grow / total_flex_factor))
                            + node.style.flex_basis.unwrap_or_else(|| node.style.width.unwrap_or(0.0))
                    };

                    // Get total flex factor
                    let mut total_flex_factor: f32 = 0.0;
                    for &id in flex_child_ids.iter() {
                        total_flex_factor += tree[id].style.flex_grow;
                    }

                    // Get list of children with a max_width
                    let mut maxable_ids = flex_child_ids
                        .iter()
                        .filter(|&&id| tree[id].style.max_width != None)
                        .copied()
                        .collect::<Vec<usize>>();

                    // Account for children that have hit their max
                    let mut just_found_maxed = true;
                    while just_found_maxed {
                        just_found_maxed = false;
                        for i in 0..maxable_ids.len() {
                            if tree[maxable_ids[i]].style.max_width?
                                < desired_width(&tree[maxable_ids[i]], remaining_space, total_flex_factor)
                            {
                                remaining_space -= tree[maxable_ids[i]].style.max_width?;
                                total_flex_factor -= tree[maxable_ids[i]].style.flex_grow;

                                maxable_ids.remove(i);
                                just_found_maxed = true;
                                break;
                            }
                        }
                    }

                    // Get list of children with a min_width
                    let mut minnable_ids = flex_child_ids
                        .iter()
                        .filter(|&&id| tree[id].style.min_width != None)
                        .copied()
                        .collect::<Vec<usize>>();

                    // Account for children that have hit their min
                    let mut just_found_minned = true;
                    while just_found_minned {
                        just_found_minned = false;
                        for i in 0..minnable_ids.len() {
                            if tree[minnable_ids[i]].style.min_width?
                                > desired_width(&tree[minnable_ids[i]], remaining_space, total_flex_factor)
                            {
                                remaining_space -= tree[minnable_ids[i]].style.min_width?;
                                total_flex_factor -= tree[minnable_ids[i]].style.flex_grow;

                                minnable_ids.remove(i);
                                just_found_minned = true;
                                break;
                            }
                        }
                    }

                    // flex-grow vs flex-shrink
                    if remaining_space >= 0.0 {
                        // If any margins are set to Auto, the space between children will flex instead
                        if num_auto_margins > 0 {
                            todo!();
                        } else {
                            // Grow children
                            for &id in flex_child_ids.iter() {
                                let child_width = desired_width(&tree[id], remaining_space, total_flex_factor)
                                    .min(tree[id].style.max_width.unwrap_or(f32::INFINITY))
                                    .max(tree[id].style.min_width.unwrap_or(0.0))
                                    + tree[id].style.border_left_width
                                    + tree[id].style.border_right_width
                                    + tree[id].style.padding_left
                                    + tree[id].style.padding_right;

                                let child_bounds = Bounds {
                                    min_width: child_width,
                                    max_width: child_width,
                                    min_height: 0.0,
                                    max_height: bounds.max_height,
                                };
                                let child_size = Self::solve_node(id, tree, child_bounds, layouts);
                                child_sizes.insert(id, child_size?);
                            }
                        }
                    } else {
                        // Get total basis and shrink factor
                        let mut total_basis: f32 = 0.0;
                        for &id in flex_child_ids.iter() {
                            total_basis += tree[id]
                                .style
                                .flex_basis
                                .unwrap_or_else(|| tree[id].style.width.unwrap_or(0.0));
                        }

                        // Shrink children
                        for &id in flex_child_ids.iter() {
                            let basis = tree[id]
                                .style
                                .flex_basis
                                .unwrap_or_else(|| tree[id].style.width.unwrap_or(0.0));
                            let child_width = (basis
                                + (((tree[id].style.flex_shrink * basis) / total_basis) * remaining_space))
                                .max(tree[id].style.min_width.unwrap_or(0.0))
                                + tree[id].style.border_left_width
                                + tree[id].style.border_right_width
                                + tree[id].style.padding_left
                                + tree[id].style.padding_right;

                            let child_bounds = Bounds {
                                min_width: child_width,
                                max_width: child_width,
                                min_height: 0.0,
                                max_height: bounds.max_height,
                            };
                            let child_size = Self::solve_node(id, tree, child_bounds, layouts);
                            child_sizes.insert(id, child_size?);
                        }
                    }

                    // TODO: Position children
                    let mut x_pos: f32 = 0.0;
                    for &child_id in child_ids.iter() {
                        let child_size = child_sizes.get(&child_id)?;
                        let child_layout = Layout {
                            size: *child_size,
                            position: Point { x: x_pos, y: 0.0 },
                        };
                        layouts[child_id] = child_layout;
                        x_pos += child_size.width;
                    }

                    // TODO: Return correct size, taking into account node's style as well as children
                    Some(Size {
                        width: x_pos.max(bounds.min_width).min(bounds.max_width),
                        height: bounds.max_height,
                    })
                }
                FlexDirection::Column | FlexDirection::ColumnReverse => todo!(),
            }
        }
    }
}

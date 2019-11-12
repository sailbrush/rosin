use std::{collections::HashMap, f32};

use crate::dom::*;
use crate::style::Style;

#[derive(Debug, Default, Copy, Clone)]
pub struct Rect {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Bounds {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

#[derive(Debug, Copy, Clone)]
pub struct Layout {
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

impl Layout {
    pub fn solve<T>(dom: &Dom<T>, styles: &[Style], size: (i32, i32)) -> Vec<Self> {
        let mut layouts = Vec::with_capacity(dom.arena.len());
        for _ in 0..dom.arena.len() {
            layouts.push(Layout::default());
        }

        let bounds = Bounds {
            min_width: size.0 as f32,
            max_width: size.0 as f32,
            min_height: size.1 as f32,
            max_height: size.1 as f32,
        };

        let mut root_layout = Self::default();
        root_layout.size = Self::layout_node(0, dom, styles, bounds, &mut layouts);
        layouts[0] = root_layout;
        layouts
    }

    fn layout_node<T>(
        id: NodeId,
        dom: &Dom<T>,
        styles: &[Style],
        bounds: Bounds,
        layouts: &mut Vec<Self>,
    ) -> Size {
        let child_ids = dom.get_children(id);
        if child_ids.is_empty() {
            Size {
                width: styles[id].width.unwrap_or(bounds.max_width),
                height: bounds.max_height,
            }
        } else {
            let mut child_sizes: HashMap<NodeId, Size> = HashMap::new();

            // Separate children into fixed and flex groups
            let mut fixed_child_ids = Vec::new();
            let mut flex_child_ids = Vec::new();
            for child_id in child_ids.iter() {
                if styles[*child_id].flex_grow == 0.0 {
                    fixed_child_ids.push(*child_id);
                } else {
                    flex_child_ids.push((*child_id, styles[*child_id].flex_grow));
                }
            }

            // Get sizes of fixed children
        // TODO If (horizontal axis)
            let fixed_bounds = Bounds {
                min_width: 0.0,
                max_width: f32::INFINITY,
                min_height: 0.0,
                max_height: bounds.max_height,
            };
            let mut remaining_space = bounds.max_width;
            for id in fixed_child_ids.iter() {
                let child_size = Self::layout_node(*id, dom, styles, fixed_bounds, layouts);
                child_sizes.insert(*id, child_size);
                remaining_space -= child_size.width;
            }

            // Get total flex
            let mut total_flex: f32 = 0.0;
            for (_, flex_factor) in flex_child_ids.iter() {
                total_flex += flex_factor;
            }

            for (id, flex_factor) in flex_child_ids.iter() {
                let flex_space = Bounds {
                    min_width: (remaining_space * (flex_factor / total_flex)).max(0.0),
                    max_width: (remaining_space * (flex_factor / total_flex)).max(0.0),
                    min_height: 0.0,
                    max_height: bounds.max_height,
                };
                let child_size = Self::layout_node(*id, dom, styles, flex_space, layouts);
                child_sizes.insert(*id, child_size);
            }

            // Position children based on layout rules
            let mut x_pos: f32 = 0.0;
            for child_id in child_ids.iter() {
                let child_size = *child_sizes.get(&child_id).unwrap();
                let child_layout = Layout {
                    size: child_size,
                    position: Point { x: x_pos, y: 0.0 },
                };
                layouts[*child_id] = child_layout;
                x_pos += child_size.width;
            }

            // Calculate this node's size based on children
            Size {
                width: x_pos,
                height: bounds.max_height,
            }
        }
    }
}

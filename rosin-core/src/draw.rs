#![forbid(unsafe_code)]

use crate::prelude::*;
use crate::tree::ArrayNode;
use crate::{geometry::Point, layout::Layout};

use bumpalo::{
    collections::{CollectIn, Vec as BumpVec},
    Bump,
};
use druid_shell::{
    kurbo,
    piet::{Piet, RenderContext},
};

pub(crate) fn draw<S, H>(temp: &Bump, state: &S, tree: &[ArrayNode<S, H>], styles: &[Style], layouts: &[Layout], piet: &mut Piet<'_>) {
    draw_inner(temp, state, tree, styles, layouts, piet, 0..1);
}

// TODO - support opacity
fn draw_inner<S, H>(
    temp: &Bump,
    state: &S,
    tree: &[ArrayNode<S, H>],
    styles: &[Style],
    layouts: &[Layout],
    piet: &mut Piet<'_>,
    range: std::ops::Range<usize>,
) {
    // Sort by z-index
    let mut ids: BumpVec<usize> = range.collect_in(temp);
    ids.sort_by(|a, b| styles[*a].z_index.partial_cmp(&styles[*b].z_index).unwrap());

    for id in ids {
        let node = &tree[id];
        let style = &styles[id];
        let pos = layouts[id].position;
        let size = layouts[id].size;
        let mask = kurbo::RoundedRect::new(
            0.0,
            0.0,
            size.width as f64,
            size.height as f64,
            (
                style.border_top_left_radius as f64,
                style.border_top_right_radius as f64,
                style.border_bottom_right_radius as f64,
                style.border_bottom_left_radius as f64,
            ),
        );

        piet.with_save(|piet| {
            piet.transform(kurbo::Affine::translate((pos.x as f64, pos.y as f64)));

            // ---------- Outset Shadows ----------
            // TODO - spread/offset
            if let Some(shadows) = &style.box_shadow {
                for shadow in shadows.iter() {
                    let blur = shadow.blur.resolve(style.font_size);
                    if blur < 1.0 {
                        piet.fill(mask, shadow.color.as_ref().unwrap_or(&style.color));
                    } else {
                        piet.blurred_rect(mask.rect(), blur, shadow.color.as_ref().unwrap_or(&style.color));
                    }
                }
            }

            // ---------- Background and Gradients ----------
            piet.fill(mask, &style.background_color);
            if let Some(gradients) = &style.background_image {
                for gradient in gradients.iter() {
                    piet.fill(mask, &gradient.resolve(size.width, size.height));
                }
            }

            // ---------- Inside Box ----------
            piet.clip(mask);

            // ---------- Inset Shadows ----------
            // TODO

            piet.with_save(|piet| {
                // ---------- Call on_draw() Callbacks ----------
                if let Some(on_draw) = &node.draw_callback {
                    piet.transform(kurbo::Affine::translate((
                        style.border_left_width as f64,
                        style.border_top_width as f64,
                    )));
                    let mut ctx = DrawCtx {
                        piet,
                        style,
                        width: size.width as f64,
                        height: size.height as f64,
                        must_draw: true, // TODO - caching system
                    };
                    (*on_draw)(state, &mut ctx);
                }
                Ok(())
            })?;

            // ---------- Border ----------
            if style.border_top_width > 0.0
                || style.border_right_width > 0.0
                || style.border_bottom_width > 0.0
                || style.border_left_width > 0.0
            {
                let mut border_mask = kurbo::BezPath::new();
                let tl: Point = (0.0, 0.0).into(); // Top Left
                let tr: Point = (size.width, 0.0).into(); // Top Right
                let br: Point = (size.width, size.height).into(); // Bottom Right
                let bl: Point = (0.0, size.height).into(); // Bottom Left
                let ctrl_points = |outer_radius: f32, h_width: f32, v_width: f32| {
                    let k = 0.552_228_45; // Kappa - magic value for approximating a circle with cubic curves
                    let pv: Point = (h_width, (outer_radius.max(v_width) - v_width) * (1.0 - k) + v_width).into();
                    let ph: Point = ((outer_radius.max(h_width) - h_width) * (1.0 - k) + h_width, v_width).into();
                    (pv, ph)
                };

                // Outside of box
                border_mask.move_to(tl + (style.border_left_width, size.height / 2.0));
                border_mask.line_to(tl + (-1.0, size.height / 2.0));
                border_mask.line_to(bl + (-1.0, 1.0));
                border_mask.line_to(br + (1.0, 1.0));
                border_mask.line_to(tr + (1.0, -1.0));
                border_mask.line_to(tl + (-1.0, -1.0));
                border_mask.line_to(tl + (-1.0, size.height / 2.0));
                border_mask.line_to(tl + (style.border_left_width, size.height / 2.0));

                // Top left corner
                let p1 = tl + (style.border_left_width, style.border_top_left_radius.max(style.border_top_width));
                let (p2, p3) = ctrl_points(style.border_top_left_radius, style.border_left_width, style.border_top_width);
                let p4 = tl + (style.border_top_left_radius.max(style.border_left_width), style.border_top_width);
                border_mask.line_to(p1);
                border_mask.curve_to(p2, p3, p4);

                // Top right corner
                let p5 = tr + (-style.border_top_right_radius.max(style.border_right_width), style.border_top_width);
                let (p7, p6) = ctrl_points(style.border_top_right_radius, style.border_right_width, style.border_top_width);
                let p6 = tr + (-p6.x, p6.y);
                let p7 = tr + (-p7.x, p7.y);
                let p8 = tr + (-style.border_right_width, style.border_top_right_radius.max(style.border_top_width));
                border_mask.line_to(p5);
                border_mask.curve_to(p6, p7, p8);

                // Bottom right corner
                let p9 = br
                    - (
                        style.border_right_width,
                        style.border_bottom_right_radius.max(style.border_bottom_width),
                    );
                let (p10, p11) = ctrl_points(
                    style.border_bottom_right_radius,
                    style.border_right_width,
                    style.border_bottom_width,
                );
                let p10 = br - p10;
                let p11 = br - p11;
                let p12 = br
                    - (
                        style.border_bottom_right_radius.max(style.border_right_width),
                        style.border_bottom_width,
                    );
                border_mask.line_to(p9);
                border_mask.curve_to(p10, p11, p12);

                // Bottom left corner
                let p13 = bl
                    + (
                        style.border_bottom_left_radius.max(style.border_left_width),
                        -style.border_bottom_width,
                    );
                let (p15, p14) = ctrl_points(style.border_bottom_left_radius, style.border_left_width, style.border_bottom_width);
                let p14 = bl + (p14.x, -p14.y);
                let p15 = bl + (p15.x, -p15.y);
                let p16 = bl
                    + (
                        style.border_left_width,
                        -style.border_bottom_left_radius.max(style.border_bottom_width),
                    );
                border_mask.line_to(p13);
                border_mask.curve_to(p14, p15, p16);
                border_mask.close_path();
                piet.clip(border_mask);

                // Lerp factors for corner points
                let f1 = if style.border_left_width >= style.border_top_left_radius {
                    1.0
                } else if style.border_top_width >= style.border_top_left_radius {
                    0.0
                } else {
                    ((style.border_left_width / style.border_top_width).min(f32::INFINITY).atan() / std::f32::consts::FRAC_PI_2)
                        .clamp(0.0, 1.0)
                };
                let f2 = if style.border_top_width >= style.border_top_right_radius {
                    1.0
                } else if style.border_right_width >= style.border_top_right_radius {
                    0.0
                } else {
                    ((style.border_top_width / style.border_right_width).min(f32::INFINITY).atan() / std::f32::consts::FRAC_PI_2)
                        .clamp(0.0, 1.0)
                };
                let f3 = if style.border_right_width >= style.border_bottom_right_radius {
                    1.0
                } else if style.border_bottom_width >= style.border_bottom_right_radius {
                    0.0
                } else {
                    ((style.border_right_width / style.border_bottom_width).min(f32::INFINITY).atan() / std::f32::consts::FRAC_PI_2)
                        .clamp(0.0, 1.0)
                };
                let f4 = if style.border_bottom_width >= style.border_bottom_left_radius {
                    1.0
                } else if style.border_left_width >= style.border_bottom_left_radius {
                    0.0
                } else {
                    ((style.border_bottom_width / style.border_left_width).min(f32::INFINITY).atan() / std::f32::consts::FRAC_PI_2)
                        .clamp(0.0, 1.0)
                };

                // Corner points that mark the boundaries between border colors
                let c1 = p1.lerp(p4, f1);
                let c2 = p5.lerp(p8, f2);
                let c3 = p9.lerp(p12, f3);
                let c4 = p13.lerp(p16, f4);

                // Top line
                if style.border_top_width > 0.0 {
                    let mut border_top = kurbo::BezPath::new();
                    border_top.move_to(tl + (-1.0, -1.0));
                    border_top.line_to(c1);
                    border_top.line_to(c2);
                    border_top.line_to(tr + (1.0, -1.0));
                    border_top.close_path();
                    piet.fill(border_top, &style.border_top_color);
                }

                // Bottom line
                if style.border_bottom_width > 0.0 {
                    let mut border_bottom = kurbo::BezPath::new();
                    border_bottom.move_to(bl + (-1.0, 1.0));
                    border_bottom.line_to(br + (1.0, 1.0));
                    border_bottom.line_to(c3);
                    border_bottom.line_to(c4);
                    border_bottom.close_path();
                    piet.fill(border_bottom, &style.border_bottom_color);
                }

                // Left line
                if style.border_left_width > 0.0 {
                    let mut border_left = kurbo::BezPath::new();
                    border_left.move_to(tl + (-1.0, -1.0));
                    border_left.line_to(c1);
                    border_left.line_to(c4);
                    border_left.line_to(bl + (-1.0, 1.0));
                    border_left.close_path();
                    piet.fill(border_left, &style.border_left_color);
                }

                // Right line
                if style.border_right_width > 0.0 {
                    let mut border_right = kurbo::BezPath::new();
                    border_right.move_to(tr + (1.0, -1.0));
                    border_right.line_to(c2);
                    border_right.line_to(c3);
                    border_right.line_to(br + (1.0, 1.0));
                    border_right.close_path();
                    piet.fill(border_right, &style.border_right_color);
                }
            }

            Ok(())
        })
        .unwrap(); // TODO - Propagate result to Viewport

        // ---------- Children ----------
        if let Some(child_ids) = node.child_ids() {
            draw_inner(temp, state, tree, styles, layouts, piet, child_ids);
        }
    }
}

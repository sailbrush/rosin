use bumpalo::{Bump, collections::Vec as BumpVec};
use kurbo::{Affine, BezPath, Rect, RoundedRectRadii, Stroke, Vec2};
use vello::{
    Scene,
    peniko::{BlendMode, Compose, Fill, Mix},
};

use crate::{
    layout::{self, TextCacheEntry},
    prelude::*,
    text,
};

/// Represents a pending operation on the draw stack.
enum DrawCommand {
    /// Draw the element (shadows/bg/border/content) and push the clip and/or opacity layers.
    Element {
        idx: usize,
        affine: Affine,
        clip_rect: Rect,
        has_opacity_ancestor: bool,
    },
    /// Pop clip or opacity layers.
    PopLayer,
}

#[allow(clippy::too_many_arguments)]
#[inline]
pub(crate) fn draw<S, H>(
    temp: &Bump,
    state: &S,
    tree: &mut Ui<S, H>,
    did_layout: bool,
    perf_info: &PerfInfo,
    scale: Vec2,
    active_node: Option<NodeId>,
    focused_node: Option<NodeId>,
    translation_map: TranslationMap,
    scene: &mut Scene,
) {
    let mut children: BumpVec<usize> = BumpVec::with_capacity_in(tree.max_children, temp);
    let mut stack: BumpVec<DrawCommand> = BumpVec::with_capacity_in(tree.nodes.len(), temp);
    let mut fixed_stack: BumpVec<DrawCommand> = BumpVec::with_capacity_in(tree.fixed_nodes.len(), temp);

    let root_clip = Affine::scale_non_uniform(scale.x, scale.y).transform_rect_bbox(tree.layout_cache[0].rect());
    let root_affine = Affine::IDENTITY.then_scale_non_uniform(scale.x, scale.y);
    stack.push(DrawCommand::Element {
        idx: 0,
        affine: root_affine,
        clip_rect: root_clip,
        has_opacity_ancestor: false,
    });

    let mut fragment = Scene::new();
    let fill = Fill::NonZero;
    let blend_mode = BlendMode::new(Mix::Normal, Compose::SrcOver);

    while let Some(cmd) = stack.pop() {
        match cmd {
            DrawCommand::Element {
                idx,
                affine,
                clip_rect,
                has_opacity_ancestor,
            } => {
                let style = &tree.style_cache[idx];
                if style.display.is_none() {
                    continue;
                }

                let rect = &tree.layout_cache[idx];

                let has_opacity_layer = (style.opacity - 1.0).abs() > f32::EPSILON;
                if has_opacity_layer {
                    scene.push_layer(fill, blend_mode, style.opacity, Affine::IDENTITY, &root_clip);
                    stack.push(DrawCommand::PopLayer);
                }

                // ---------- Fixed Children ----------
                // We trap position: fixed elements if any of its ancestors have an opacity less than one.
                let should_trap_fixed = has_opacity_layer || has_opacity_ancestor;
                tree.child_indexes(idx, &mut children);
                children.sort_by_key(|&a| tree.style_cache[a].z_index);
                for &child_idx in children.iter().rev() {
                    let child_style = &tree.style_cache[child_idx];
                    if child_style.position == Position::Fixed {
                        let child_affine = root_affine * child_style.transform;
                        if should_trap_fixed {
                            stack.push(DrawCommand::Element {
                                idx: child_idx,
                                affine: child_affine,
                                clip_rect: root_clip,
                                has_opacity_ancestor: true,
                            });
                        } else {
                            fixed_stack.push(DrawCommand::Element {
                                idx: child_idx,
                                affine: child_affine,
                                clip_rect: root_clip,
                                has_opacity_ancestor: false,
                            });
                        }
                    }
                }

                // ---------- Box Shadows ----------
                if let Some(shadows) = &style.box_shadow {
                    for shadow in shadows.iter() {
                        if shadow.inset {
                            continue;
                        }

                        let spread = shadow.spread.resolve(style.font_size) as f64;
                        let blur = shadow.blur.resolve(style.font_size) as f64;
                        let offset_x = shadow.offset_x.resolve(style.font_size) as f64;
                        let offset_y = shadow.offset_y.resolve(style.font_size) as f64;

                        let extent = spread + blur;
                        let shadow_box = rect.rect().inflate(extent, extent);
                        let shadow_affine = affine * Affine::translate((offset_x, offset_y));
                        let global_shadow_box = shadow_affine.transform_rect_bbox(shadow_box);

                        let shadow_intersection = clip_rect.intersect(global_shadow_box);
                        if shadow_intersection.width() <= 0.0 || shadow_intersection.height() <= 0.0 {
                            continue;
                        }

                        if blur == 0.0 {
                            let rr = rect.rect().inflate(spread, spread);
                            // Negative spread can shrink the rect to nothing
                            if rr.width() <= 0.0 || rr.height() <= 0.0 {
                                continue;
                            }

                            // Negative spread can make radii negative
                            let base = rect.radii();
                            let tl = (base.top_left + spread).max(0.0);
                            let tr = (base.top_right + spread).max(0.0);
                            let bl = (base.bottom_left + spread).max(0.0);
                            let br = (base.bottom_right + spread).max(0.0);

                            let radii = RoundedRectRadii::new(tl, tr, bl, br);
                            let shadow_rounded_rect = rr.to_rounded_rect(radii);

                            scene.fill(fill, shadow_affine, shadow.color.unwrap_or(style.color), None, &shadow_rounded_rect);
                        } else {
                            let rr = rect.rect().inflate(spread, spread);
                            if rr.width() <= 0.0 || rr.height() <= 0.0 {
                                continue;
                            }

                            let base = rect.radii();
                            let tl = (base.top_left + spread).max(0.0);
                            let tr = (base.top_right + spread).max(0.0);
                            let br = (base.bottom_right + spread).max(0.0);
                            let bl = (base.bottom_left + spread).max(0.0);

                            let shadow_color = shadow.color.unwrap_or(style.color);
                            let blur = blur / 2.0;

                            let eps = 1e-4;
                            let all_same = (tl - tr).abs() <= eps && (tl - br).abs() <= eps && (tl - bl).abs() <= eps;

                            if all_same {
                                scene.draw_blurred_rounded_rect(shadow_affine, rr, shadow_color, tl, blur);
                            } else {
                                let mid_x = (rr.x0 + rr.width() * 0.5).round();
                                let mid_y = (rr.y0 + rr.height() * 0.5).round();
                                let pad = (blur * 2.0) + 2.0;

                                // Quantize the outer clip bounds to logical pixels
                                let x0 = (rr.x0 - pad).floor();
                                let y0 = (rr.y0 - pad).floor();
                                let x1 = (rr.x1 + pad).ceil();
                                let y1 = (rr.y1 + pad).ceil();

                                // Top-left corner
                                let clip = Rect::new(x0, y0, mid_x, mid_y);
                                scene.push_layer(fill, blend_mode, 1.0, shadow_affine, &clip);
                                scene.draw_blurred_rounded_rect(shadow_affine, rr, shadow_color, tl, blur);
                                scene.pop_layer();

                                // Top-right corner
                                let clip = Rect::new(mid_x, y0, x1, mid_y);
                                scene.push_layer(fill, blend_mode, 1.0, shadow_affine, &clip);
                                scene.draw_blurred_rounded_rect(shadow_affine, rr, shadow_color, tr, blur);
                                scene.pop_layer();

                                // Bottom-right corner
                                let clip = Rect::new(mid_x, mid_y, x1, y1);
                                scene.push_layer(fill, blend_mode, 1.0, shadow_affine, &clip);
                                scene.draw_blurred_rounded_rect(shadow_affine, rr, shadow_color, br, blur);
                                scene.pop_layer();

                                // Bottom-left corner
                                let clip = Rect::new(x0, mid_y, mid_x, y1);
                                scene.push_layer(fill, blend_mode, 1.0, shadow_affine, &clip);
                                scene.draw_blurred_rounded_rect(shadow_affine, rr, shadow_color, bl, blur);
                                scene.pop_layer();
                            }
                        }
                    }
                }

                let visible_rect = clip_rect.intersect(affine.transform_rect_bbox(rect.rect()));
                if visible_rect.width() > 0.0 && visible_rect.height() > 0.0 {
                    let content_transform = Affine::translate(rect.origin().to_vec2());

                    // ---------- Outline ----------
                    let outline_width: f64 = style.outline_width.resolve(style.font_size) as f64;
                    if outline_width > 0.0 {
                        let outline_color = style.outline_color;
                        let offset: f64 = style.outline_offset.resolve(style.font_size) as f64;
                        let bounding_box = rect.rect();
                        let size = bounding_box.size().to_vec2() + Vec2::new(1.0 + offset, 1.0 + offset);
                        let outline = Rect::from_center_size(bounding_box.center(), size.to_size()).to_rounded_rect(rect.radii());
                        scene.stroke(&Stroke::new(outline_width), affine, outline_color, None, &outline);
                    }

                    // ---------- Background Gradients ----------
                    scene.fill(fill, affine, style.background_color, None, &rect);
                    if let Some(gradients) = &style.background_image {
                        for gradient in gradients.stack.iter().rev() {
                            scene.fill(fill, affine, &gradient.resolve(rect.rect(), style.color), None, &rect);
                        }
                    }

                    // ----------- Clip Layer ----------
                    scene.push_clip_layer(fill, affine, &rect);

                    // ---------- Border ----------
                    let border_top_width = style.border_top_width.resolve(style.font_size) as f64;
                    let border_right_width = style.border_right_width.resolve(style.font_size) as f64;
                    let border_bottom_width = style.border_bottom_width.resolve(style.font_size) as f64;
                    let border_left_width = style.border_left_width.resolve(style.font_size) as f64;

                    let has_border = border_top_width > 0.0 || border_right_width > 0.0 || border_bottom_width > 0.0 || border_left_width > 0.0;

                    if has_border {
                        let same_colors = style.border_top_color == style.border_right_color
                            && style.border_right_color == style.border_bottom_color
                            && style.border_bottom_color == style.border_left_color;

                        let same_widths =
                            border_top_width == border_right_width && border_right_width == border_bottom_width && border_bottom_width == border_left_width;

                        if same_colors && same_widths {
                            // This stroke is clipped by the clip layer we just pushed.
                            scene.stroke(&Stroke::new(border_top_width * 2.0), affine, style.border_top_color, None, &tree.layout_cache[idx]);
                        } else {
                            let mut border_mask = BezPath::new();
                            let tl: Vec2 = (0.0, 0.0).into();
                            let tr: Vec2 = (rect.width(), 0.0).into();
                            let br: Vec2 = (rect.width(), rect.height()).into();
                            let bl: Vec2 = (0.0, rect.height()).into();

                            let ctrl_points = |outer_radius: f64, h_width: f64, v_width: f64| {
                                let k = 0.552_228_45;
                                let pv: Vec2 = (h_width, (outer_radius.max(v_width) - v_width) * (1.0 - k) + v_width).into();
                                let ph: Vec2 = ((outer_radius.max(h_width) - h_width) * (1.0 - k) + h_width, v_width).into();
                                (pv, ph)
                            };

                            // Outside of box
                            border_mask.move_to((tl + Vec2::new(border_left_width, rect.height() / 2.0)).to_point());
                            border_mask.line_to((tl + Vec2::new(-1.0, rect.height() / 2.0)).to_point());
                            border_mask.line_to((bl + Vec2::new(-1.0, 1.0)).to_point());
                            border_mask.line_to((br + Vec2::new(1.0, 1.0)).to_point());
                            border_mask.line_to((tr + Vec2::new(1.0, -1.0)).to_point());
                            border_mask.line_to((tl + Vec2::new(-1.0, -1.0)).to_point());
                            border_mask.line_to((tl + Vec2::new(-1.0, rect.height() / 2.0)).to_point());
                            border_mask.line_to((tl + Vec2::new(border_left_width, rect.height() / 2.0)).to_point());

                            // Top left corner
                            let p1: Vec2 = tl + Vec2::new(border_left_width, rect.radii().top_left.max(border_top_width));
                            let (p2, p3) = ctrl_points(rect.radii().top_left, border_left_width, border_top_width);
                            let p4: Vec2 = tl + Vec2::new(rect.radii().top_left.max(border_left_width), border_top_width);
                            border_mask.line_to(p1.to_point());
                            border_mask.curve_to(p2.to_point(), p3.to_point(), p4.to_point());

                            // Top right corner
                            let p5: Vec2 = tr + Vec2::new(-rect.radii().top_right.max(border_right_width), border_top_width);
                            let (p7, p6) = ctrl_points(rect.radii().top_right, border_right_width, border_top_width);
                            let p6: Vec2 = tr + Vec2::new(-p6.x, p6.y);
                            let p7: Vec2 = tr + Vec2::new(-p7.x, p7.y);
                            let p8: Vec2 = tr + Vec2::new(-border_right_width, rect.radii().top_right.max(border_top_width));
                            border_mask.line_to(p5.to_point());
                            border_mask.curve_to(p6.to_point(), p7.to_point(), p8.to_point());

                            // Bottom right corner
                            let p9: Vec2 = br - (border_right_width, rect.radii().bottom_right.max(border_bottom_width)).into();
                            let (p10, p11) = ctrl_points(rect.radii().bottom_right, border_right_width, border_bottom_width);
                            let p10: Vec2 = br - p10;
                            let p11: Vec2 = br - p11;
                            let p12: Vec2 = br - (rect.radii().bottom_right.max(border_right_width), border_bottom_width).into();
                            border_mask.line_to(p9.to_point());
                            border_mask.curve_to(p10.to_point(), p11.to_point(), p12.to_point());

                            // Bottom left corner
                            let p13: Vec2 = bl + Vec2::new(rect.radii().bottom_left.max(border_left_width), -border_bottom_width);
                            let (p15, p14) = ctrl_points(rect.radii().bottom_left, border_left_width, border_bottom_width);
                            let p14: Vec2 = bl + Vec2::new(p14.x, -p14.y);
                            let p15: Vec2 = bl + Vec2::new(p15.x, -p15.y);
                            let p16: Vec2 = bl + Vec2::new(border_left_width, -rect.radii().bottom_left.max(border_bottom_width));
                            border_mask.line_to(p13.to_point());
                            border_mask.curve_to(p14.to_point(), p15.to_point(), p16.to_point());
                            border_mask.close_path();

                            // Clip border
                            scene.push_layer(fill, blend_mode, 1.0, affine * content_transform, &border_mask);

                            if same_colors {
                                scene.fill(fill, affine, style.border_top_color, None, &rect);
                            } else {
                                let lerp_factor = |a: f64, b: f64, r: f64| {
                                    if r != 0.0 { ((a / (a + b)) + (a / r) - (b / r)).clamp(0.0, 1.0) } else { 0.0 }
                                };

                                let f1 = lerp_factor(border_left_width, border_top_width, rect.radii().top_left);
                                let f2 = lerp_factor(border_top_width, border_right_width, rect.radii().top_right);
                                let f3 = lerp_factor(border_right_width, border_bottom_width, rect.radii().bottom_right);
                                let f4 = lerp_factor(border_bottom_width, border_left_width, rect.radii().bottom_left);

                                // Corner points that mark the boundaries between border colors
                                let c1 = p1.lerp(p4, f1);
                                let c2 = p5.lerp(p8, f2);
                                let c3 = p9.lerp(p12, f3);
                                let c4 = p13.lerp(p16, f4);

                                // Top line
                                if border_top_width > 0.0 {
                                    let mut border_top_path = BezPath::new();
                                    border_top_path.move_to((tl + Vec2::new(-1.0, -1.0)).to_point());
                                    border_top_path.line_to(c1.to_point());
                                    border_top_path.line_to(c2.to_point());
                                    border_top_path.line_to((tr + Vec2::new(1.0, -1.0)).to_point());
                                    border_top_path.close_path();
                                    scene.fill(fill, affine * content_transform, style.border_top_color, None, &border_top_path);
                                }

                                // Bottom line
                                if border_bottom_width > 0.0 {
                                    let mut border_bottom_path = BezPath::new();
                                    border_bottom_path.move_to((bl + Vec2::new(-1.0, 1.0)).to_point());
                                    border_bottom_path.line_to((br + Vec2::new(1.0, 1.0)).to_point());
                                    border_bottom_path.line_to(c3.to_point());
                                    border_bottom_path.line_to(c4.to_point());
                                    border_bottom_path.close_path();
                                    scene.fill(fill, affine * content_transform, style.border_bottom_color, None, &border_bottom_path);
                                }

                                // Left line
                                if border_left_width > 0.0 {
                                    let mut border_left_path = BezPath::new();
                                    border_left_path.move_to((tl + Vec2::new(-1.0, -1.0)).to_point());
                                    border_left_path.line_to(c1.to_point());
                                    border_left_path.line_to(c4.to_point());
                                    border_left_path.line_to((bl + Vec2::new(-1.0, 1.0)).to_point());
                                    border_left_path.close_path();
                                    scene.fill(fill, affine * content_transform, style.border_left_color, None, &border_left_path);
                                }

                                // Right line
                                if border_right_width > 0.0 {
                                    let mut border_right_path = BezPath::new();
                                    border_right_path.move_to((tr + Vec2::new(1.0, -1.0)).to_point());
                                    border_right_path.line_to(c2.to_point());
                                    border_right_path.line_to(c3.to_point());
                                    border_right_path.line_to((br + Vec2::new(1.0, 1.0)).to_point());
                                    border_right_path.close_path();
                                    scene.fill(fill, affine * content_transform, style.border_right_color, None, &border_right_path);
                                }
                            }

                            scene.pop_layer(); // border clip
                        }
                    }

                    // ---------- Inset Shadows ----------
                    // TODO - Waiting on Vello support for blurs

                    // ---------- Call Canvas Callback ----------
                    let node = &mut tree.nodes[idx];
                    let is_enabled = node.enabled.get_or(true);
                    if let Some(on_draw) = &mut node.canvas_callback {
                        fragment.reset();
                        let mut ctx = CanvasCtx {
                            did_layout,
                            is_active: active_node.is_some() && active_node == node.nid,
                            is_focused: focused_node.is_some() && focused_node == node.nid,
                            is_enabled,
                            perf_info,
                            rect,
                            scene: &mut fragment,
                            style,
                            translation_map: translation_map.clone(),
                        };
                        (*on_draw)(state, &mut ctx);

                        scene.append(&fragment, Some(affine * content_transform));
                    }

                    // ---------- Draw Text ----------
                    if let Some(text) = &node.text {
                        let max_width = layout::max_content_width(style, rect);
                        let font_style = style.get_font_layout_style();

                        if let Some(cache) = tree.text_cache.get_mut(&idx) {
                            // even if we use the cache, we still depend on these vars
                            cache.deps.mark_read();
                            // if the text has changed since last time it was laid out, we need to redo that
                            if (cache.deps.any_changed_update() || cache.font_style != font_style)
                                && let Some(resolved) = text.resolve(&translation_map)
                            {
                                cache.layout = text::layout_text(&font_style, Some(max_width), &resolved);
                                cache.font_style = font_style;
                                cache.max_width = Some(max_width);

                            // else if wrap_width changed more than a logical pixel, re-wrap.
                            } else if match (cache.max_width, max_width) {
                                (Some(a), b) => (a - b).abs() > 1.0,
                                _ => true,
                            } {
                                cache.layout.break_all_lines(Some(max_width));
                                cache.max_width = Some(max_width);
                            }

                            let origin = layout::align_and_position_text(style, rect, &mut cache.layout);
                            fragment.reset();
                            text::draw_text(&mut fragment, style, origin, &cache.layout);
                            scene.append(&fragment, Some(affine * content_transform));
                        } else {
                            // No cache entry, need to resolve string, cache the result, and draw
                            let mut layout = None;
                            let deps = DependencyMap::default().read_scope(|| {
                                if let Some(resolved) = text.resolve(&translation_map) {
                                    layout = Some(text::layout_text(&font_style, Some(max_width), &resolved));
                                }
                            });

                            if let Some(mut layout) = layout {
                                let origin = layout::align_and_position_text(style, rect, &mut layout);
                                fragment.reset();
                                text::draw_text(&mut fragment, style, origin, &layout);
                                scene.append(&fragment, Some(affine * content_transform));

                                let entry = TextCacheEntry {
                                    deps,
                                    layout,
                                    font_style,
                                    max_width: Some(max_width),
                                };
                                tree.text_cache.insert(idx, entry);
                            }
                        }
                    }

                    // Pop the clip layer after children (LIFO)
                    stack.push(DrawCommand::PopLayer);

                    // ---------- Normal Children ----------
                    for &child_idx in children.iter().rev() {
                        let child_style = &tree.style_cache[child_idx];
                        if child_style.position != Position::Fixed {
                            let child_affine = affine * child_style.transform;
                            stack.push(DrawCommand::Element {
                                idx: child_idx,
                                affine: child_affine,
                                clip_rect: visible_rect,
                                has_opacity_ancestor: should_trap_fixed,
                            });
                        }
                    }
                } else {
                    // Not visible: still push normal children so they can be culled independently.
                    for &child_idx in children.iter().rev() {
                        let child_style = &tree.style_cache[child_idx];
                        if child_style.position != Position::Fixed {
                            let child_affine = affine * child_style.transform;
                            stack.push(DrawCommand::Element {
                                idx: child_idx,
                                affine: child_affine,
                                clip_rect: visible_rect,
                                has_opacity_ancestor: should_trap_fixed,
                            });
                        }
                    }
                }
            }
            DrawCommand::PopLayer => {
                scene.pop_layer();
            }
        }

        if stack.is_empty() {
            stack.extend(fixed_stack.drain(..));
        }
    }
}

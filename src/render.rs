#![forbid(unsafe_code)]

use crate::layout::Layout;
use crate::prelude::*;
use crate::tree::ArrayNode;

use femtovg::{Canvas, FontId, Paint, Path, Renderer};

pub(crate) fn render<T, R: Renderer>(tree: &[ArrayNode<T>], layout: &[Layout], canvas: &mut Canvas<R>, font_table: &[(u32, FontId)]) {
    render_node(tree, layout, 0, 0.0, 0.0, canvas, font_table);
}

fn render_node<T, R: Renderer>(
    tree: &[ArrayNode<T>],
    layout: &[Layout],
    id: usize,
    offset_x: f32,
    offset_y: f32,
    canvas: &mut Canvas<R>,
    font_table: &[(u32, FontId)],
) {
    if layout[id].size.width != 0.0 && layout[id].size.height != 0.0 {
        // Draw the box
        let bg_color = tree[id].style.background_color;
        let fill_paint = Paint::color(femtovg::Color::rgba(bg_color.red, bg_color.green, bg_color.blue, bg_color.alpha));

        // TODO - use all border colors
        let border_color = tree[id].style.border_top_color;
        let mut border_paint = Paint::color(femtovg::Color::rgba(
            border_color.red,
            border_color.green,
            border_color.blue,
            border_color.alpha,
        ));
        border_paint.set_line_width(tree[id].style.border_top_width);

        let mut path = Path::new();
        path.rounded_rect_varying(
            layout[id].position.x + offset_x,
            layout[id].position.y + offset_y,
            layout[id].size.width,
            layout[id].size.height,
            tree[id].style.border_top_left_radius,
            tree[id].style.border_top_right_radius,
            tree[id].style.border_bottom_right_radius,
            tree[id].style.border_bottom_left_radius,
        );
        canvas.fill_path(&mut path, fill_paint);
        canvas.stroke_path(&mut path, border_paint);

        // Draw text
        if let Content::Label(text) = tree[id].content {
            let font_family = &tree[id].style.font_family;
            let (_, font_id) = font_table
                .iter()
                .find(|(name, _)| *name == *font_family)
                .expect("[Rosin] Font not found");

            let font_color = tree[id].style.color;
            let mut paint = Paint::color(femtovg::Color::rgba(
                font_color.red,
                font_color.green,
                font_color.blue,
                font_color.alpha,
            ));
            paint.set_font_size(tree[id].style.font_size);
            paint.set_font(&[*font_id]);
            paint.set_text_align(femtovg::Align::Left);
            paint.set_text_baseline(femtovg::Baseline::Top);
            let _ = canvas.fill_text(
                layout[id].position.x + offset_x + tree[id].style.padding_left + tree[id].style.border_left_width,
                layout[id].position.y + offset_y + tree[id].style.padding_top + tree[id].style.border_top_width,
                text,
                paint,
            );
        }
    }

    for i in tree[id].child_ids() {
        render_node(
            tree,
            layout,
            i,
            layout[id].position.x + offset_x,
            layout[id].position.y + offset_y,
            canvas,
            font_table,
        );
    }
}

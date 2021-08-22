#![forbid(unsafe_code)]

use crate::layout::Layout;
use crate::tree::ArrayNode;

use femtovg::{renderer::OpenGl, Canvas, FontId, Paint, Path};

pub(crate) fn render<T>(
    state: &T,
    tree: &mut [ArrayNode<T>],
    layout: &[Layout],
    canvas: &mut Canvas<OpenGl>,
    font_table: &[(u32, FontId)],
) {
    render_node(state, tree, layout, 0, 0.0, 0.0, canvas, font_table);
}

fn render_node<T>(
    state: &T,
    tree: &mut [ArrayNode<T>],
    layout: &[Layout],
    id: usize,
    offset_x: f32,
    offset_y: f32,
    canvas: &mut Canvas<OpenGl>,
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

        if let Some(on_draw) = &mut tree[id].on_draw {
            on_draw(state, layout[id].size, canvas);
        }
    }

    for i in tree[id].child_ids() {
        render_node(
            state,
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

#![forbid(unsafe_code)]

use crate::layout::Layout;
use crate::style::Style;
use crate::tree::ArrayNode;

use femtovg::{renderer::OpenGl, Canvas, FontId, Paint, Path};

pub struct DrawCtx<'a> {
    pub canvas: &'a mut Canvas<OpenGl>,
    pub font_table: &'a [(u32, FontId)], // TODO - make a struct for this
    pub style: &'a Style,
    pub width: f32,
    pub height: f32,
    pub must_draw: bool,
}

pub(crate) fn render<T>(state: &T, tree: &[ArrayNode<T>], layout: &[Layout], canvas: &mut Canvas<OpenGl>, font_table: &[(u32, FontId)]) {
    render_node(state, tree, layout, 0, (0.0, 0.0), canvas, font_table);
}

fn render_node<T>(
    state: &T,
    tree: &[ArrayNode<T>],
    layout: &[Layout],
    id: usize,
    offset: (f32, f32),
    canvas: &mut Canvas<OpenGl>,
    font_table: &[(u32, FontId)],
) {
    if layout[id].size.width != 0.0 && layout[id].size.height != 0.0 {
        let style = &tree[id].style;

        // Draw the box
        let bg_color = style.background_color;
        let fill_paint = Paint::color(femtovg::Color::rgba(bg_color.red, bg_color.green, bg_color.blue, bg_color.alpha));

        // TODO - use all border colors
        let border_color = style.border_top_color;
        let mut border_paint = Paint::color(femtovg::Color::rgba(
            border_color.red,
            border_color.green,
            border_color.blue,
            border_color.alpha,
        ));
        border_paint.set_line_width(style.border_top_width);

        let mut path = Path::new();
        path.rounded_rect_varying(
            layout[id].position.x + offset.0,
            layout[id].position.y + offset.1,
            layout[id].size.width,
            layout[id].size.height,
            style.border_top_left_radius,
            style.border_top_right_radius,
            style.border_bottom_right_radius,
            style.border_bottom_left_radius,
        );
        canvas.fill_path(&mut path, fill_paint);
        canvas.stroke_path(&mut path, border_paint);

        // Call on_draw()
        if let Some(on_draw) = &tree[id].draw_callback {
            canvas.translate(
                layout[id].position.x + offset.0 + style.border_left_width,
                layout[id].position.y + offset.1 + style.border_top_width,
            );
            canvas.scissor(
                0.0,
                0.0,
                layout[id].size.width - style.border_left_width - style.border_right_width,
                layout[id].size.height - style.border_top_width - style.border_bottom_width,
            );

            let mut ctx = DrawCtx {
                canvas,
                font_table,
                style,
                width: layout[id].size.width,
                height: layout[id].size.height,
                must_draw: true, // TODO - caching system
            };
            (*on_draw)(state, &mut ctx);

            canvas.reset_scissor();
            canvas.reset_transform();
        }
    }

    for i in tree[id].child_ids() {
        render_node(
            state,
            tree,
            layout,
            i,
            (layout[id].position.x + offset.0, layout[id].position.y + offset.1),
            canvas,
            font_table,
        );
    }
}

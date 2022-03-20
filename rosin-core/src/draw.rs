#![forbid(unsafe_code)]

use crate::layout::Layout;
use crate::prelude::*;
use crate::tree::ArrayNode;

use druid_shell::{
    kurbo::{Affine, RoundedRect},
    piet::{Color, Piet, RenderContext},
};

pub(crate) fn draw<S, H>(state: &S, tree: &[ArrayNode<S, H>], layout: &[Layout], piet: &mut Piet<'_>) {
    draw_inner(state, tree, layout, 0, piet);
}

fn draw_inner<S, H>(state: &S, tree: &[ArrayNode<S, H>], layout: &[Layout], id: usize, piet: &mut Piet<'_>) {
    if layout[id].size.width != 0.0 && layout[id].size.height != 0.0 {
        let style = &tree[id].style;

        // ---------- Draw the box ----------
        let bg_color = Color::rgba8(
            style.background_color.red,
            style.background_color.green,
            style.background_color.blue,
            style.background_color.alpha,
        );

        // TODO - use all border colors
        let border_color = style.border_top_color;
        let border_color = Color::rgba8(border_color.red, border_color.green, border_color.blue, border_color.alpha);

        let path = RoundedRect::new(
            layout[id].position.x as f64,
            layout[id].position.y as f64,
            layout[id].position.x as f64 + layout[id].size.width as f64,
            layout[id].position.y as f64 + layout[id].size.height as f64,
            (
                style.border_top_left_radius.into(),
                style.border_top_right_radius.into(),
                style.border_bottom_right_radius.into(),
                style.border_bottom_left_radius.into(),
            ),
        );

        piet.fill(path, &bg_color);
        piet.stroke(path, &border_color, style.border_top_width.into());

        // Call on_draw()
        if let Some(on_draw) = &tree[id].draw_callback {
            piet.save().unwrap();
            piet.clip(path);

            piet.transform(Affine::translate((
                layout[id].position.x as f64 + style.border_left_width as f64,
                layout[id].position.y as f64 + style.border_top_width as f64,
            )));

            let mut ctx = DrawCtx {
                piet,
                style,
                width: layout[id].size.width as f64,
                height: layout[id].size.height as f64,
                must_draw: true, // TODO - caching system
            };

            (*on_draw)(state, &mut ctx);

            piet.restore().unwrap();
        }

        piet.stroke(path, &border_color, style.border_top_width.into());
    }

    for i in tree[id].child_ids() {
        draw_inner(state, tree, layout, i, piet);
    }
}

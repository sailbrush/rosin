#![forbid(unsafe_code)]

use crate::prelude::*;

// ---------- Static Label ----------
// TODO - need to make one that can be used from inside widgets, because new_key!() might not work
#[track_caller]
pub fn label<T>(text: &'static str) -> Node<T> {
    ui!([
        .key(new_key!())
        .on_draw(true, move |_: &T, ctx: &mut DrawCtx| {
            if !ctx.must_draw { return }

            let font_family = ctx.style.font_family;
            let (_, font_id) = ctx.font_table
                .iter()
                .find(|(name, _)| *name == font_family)
                .expect("[Rosin] Font not found");

            let font_color = ctx.style.color;
            let mut paint = Paint::color(femtovg::Color::rgba(
                font_color.red,
                font_color.green,
                font_color.blue,
                font_color.alpha,
            ));
            paint.set_font_size(ctx.style.font_size);
            paint.set_font(&[*font_id]);
            paint.set_text_align(femtovg::Align::Left);
            paint.set_text_baseline(femtovg::Baseline::Top);
            let _ = ctx.canvas.fill_text(
                ctx.style.padding_left,
                ctx.style.padding_top,
                text,
                paint,
            );
        })
    ])
}

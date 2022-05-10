#![forbid(unsafe_code)]

use druid_shell::piet::{FontFamily, RenderContext, Text, TextLayoutBuilder};

use crate::prelude::*;

// ---------- Static Label ----------
pub fn label<S, H>(text: &'static str) -> View<S, H> {
    ui!([
        .on_draw(true, move |_: &S, ctx: &mut DrawCtx| {
            let font_color = ctx.style.color.clone();

            let font_family = if let Some(family_name) = &ctx.style.font_family {
                ctx.piet.text().font_family(family_name.as_ref())
            } else {
                None
            };
            let font_family = font_family.unwrap_or(FontFamily::SYSTEM_UI);

            let layout = ctx.piet
                .text()
                .new_text_layout(text)
                .font(font_family, ctx.style.font_size as f64)
                .text_color(font_color)
                .build()
                .unwrap();

            ctx.piet.draw_text(&layout, (ctx.style.padding_left as f64, ctx.style.padding_top as f64));
        })
    ])
}

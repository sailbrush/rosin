#![forbid(unsafe_code)]

use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    sync::Arc,
};

use druid_shell::piet::{Color, FontFamily, RenderContext, Text, TextLayoutBuilder};

use crate::prelude::*;

// ---------- Dynamic Label ----------
#[derive(Debug)]
pub struct DynLabel {
    key: Key,
    text: RefCell<String>,
    changed: Cell<bool>,
}

impl DynLabel {
    pub fn new(text: &str) -> Grc<Self> {
        Grc::new(Self {
            key: Key::new(),
            text: RefCell::new(text.to_owned()),
            changed: Cell::new(false),
        })
    }

    pub fn set_text(&self, new_text: &str) -> Phase {
        let mut text = self.text.borrow_mut();
        text.clear();
        text.push_str(new_text);
        self.changed.replace(true);
        Phase::Draw
    }
}

impl Grc<DynLabel> {
    pub fn view<S>(&self) -> Node<S> {
        let this = Grc::downgrade(self);

        ui!("test" [
            .key(self.key)
            .on_draw(true, move |_, ctx: &mut DrawCtx| {
                // If the underlying data is gone, then just return since there's nothing to draw.
                // TODO: Maybe log something?
                //       Could also draw the cache, if available.
                let this = if let Some(this) = this.upgrade() { this } else { return };
                if !this.changed.get() && !ctx.must_draw { return }
                this.changed.set(false);

                let font_color = Color::rgba8(
                    ctx.style.color.red,
                    ctx.style.color.green,
                    ctx.style.color.blue,
                    ctx.style.color.alpha
                );

                let font_family = if let Some(family_name) = &ctx.style.font_family {
                    ctx.piet.text().font_family(family_name.as_ref())
                } else {
                    None
                };
                let font_family = font_family.unwrap_or(FontFamily::SANS_SERIF);

                let layout = ctx.piet
                    .text()
                    .new_text_layout(Arc::new(this.text.borrow().clone()))
                    .font(font_family, ctx.style.font_size as f64)
                    .text_color(font_color)
                    .build()
                    .unwrap();

                ctx.piet.draw_text(&layout, (ctx.style.padding_left as f64, ctx.style.padding_top as f64));
            })
        ])
    }
}

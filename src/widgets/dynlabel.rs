#![forbid(unsafe_code)]

use std::{cell::Cell, fmt::Debug};

use crate::prelude::*;

// ---------- Dynamic Label ----------
#[derive(Debug)]
pub struct DynLabel {
    key: Key,
    text: String,
    changed: Cell<bool>,
}

impl DynLabel {
    #[track_caller]
    pub fn new(text: &str) -> Self {
        Self {
            key: new_key!(),
            text: text.to_owned(),
            changed: Cell::new(false),
        }
    }

    pub fn set_text(&mut self, new_text: &str) -> Stage {
        self.text.clear();
        self.text.push_str(new_text);
        self.changed.replace(true);
        Stage::Draw
    }

    // We construct the lens each time we view the widget to avoid storing references in the tree.
    pub fn view<T>(&self, lens: impl Lens<In = T, Out = Self> + 'static) -> Node<T> {
        ui!([
            .key(self.key)
            .on_draw(false,
                move |t: &T, ctx: &mut DrawCtx| {
                    let this = lens.get(t);
                    if !this.changed.get() && !ctx.must_draw { return }
                    this.changed.replace(false);

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
                        &this.text,
                        paint,
                    );
                })
        ])
    }
}

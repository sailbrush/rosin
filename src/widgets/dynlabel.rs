#![forbid(unsafe_code)]

use std::{cell::Cell, fmt::Debug, rc::Rc};

use crate::prelude::*;

// ---------- Dynamic Label ----------
//#[derive(Debug)]
pub struct DynLabel<T> {
    key: Key,
    lens: Strong<Box<dyn Lens<T, Self>>>,
    text: String,
    changed: Cell<bool>,
}

impl<T> DynLabel<T> {
    pub fn new(text: &str, lens: impl Lens<T, Self> + 'static) -> Self {
        Self {
            key: Key::new(),
            lens: Strong::new(Box::new(lens)),
            text: text.to_owned(),
            changed: Cell::new(false),
        }
    }

    pub fn set_text(&mut self, new_text: &str) -> Phase {
        self.text.clear();
        self.text.push_str(new_text);
        self.changed.replace(true);
        Phase::Draw
    }

    pub fn view(&self) -> Node<T> {
        let lens = Strong::downgrade(&self.lens);

        ui!("dynlabel" [
            .key(self.key)
            .on_draw(false, move |t: &T, ctx: &mut DrawCtx| {
                let this = lens.upgrade().unwrap().get_ref(t);
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

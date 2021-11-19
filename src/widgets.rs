#![forbid(unsafe_code)]

use std::{cell::Cell, fmt::Debug};

use crate::prelude::*;

// ---------- Button ----------
#[track_caller]
pub fn button<T>(text: &'static str, callback: impl Fn(&mut T, &mut EventCtx) -> Stage + 'static) -> Node<T> {
    label(text).event(On::MouseDown, callback)
}

// ---------- Static Label ----------
// TODO - need to make one that can be used from inside widgets, because new_key!() might not work
#[track_caller]
pub fn label<T>(text: &'static str) -> Node<T> {
    ui!([
        .key(new_key!())
        .on_draw(true,
            move |_: &T, ctx: &mut DrawCtx| {
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

// ---------- Dynamic Label ----------
#[derive(Debug)]
pub struct DynLabel<T: 'static> {
    lens: &'static dyn Lens<In = T, Out = Self>,
    key: Key,
    text: String,
    has_changed: Cell<bool>,
}

impl<T> DynLabel<T> {
    #[track_caller]
    pub fn new(lens: impl Lens<In = T, Out = Self>, text: &str) -> Self {
        Self {
            lens: lens.leak(),
            key: new_key!(),
            text: text.to_owned(),
            has_changed: Cell::new(false),
        }
    }

    pub fn set_text(&mut self, new_text: &str) -> Stage {
        self.text.clear();
        self.text.push_str(new_text);
        self.has_changed.replace(true);
        Stage::Draw
    }

    pub fn view(&self) -> Node<T> {
        let lens = self.lens;

        ui!("example" [
            .key(self.key)
            .on_draw(false,
                move |t: &T, ctx: &mut DrawCtx| {
                    let this = lens.get(t);
                    if !this.has_changed.get() && !ctx.must_draw { return }
                    this.has_changed.replace(false);

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

// ---------- Slider ----------
#[derive(Debug)]
pub struct Slider<T: 'static> {
    lens: &'static dyn Lens<In = T, Out = Self>,
    key: Key,
    value: f32,
}

impl<T> Slider<T> {
    #[track_caller]
    pub fn new(lens: impl Lens<In = T, Out = Self>) -> Self {
        Self {
            lens: lens.leak(),
            key: new_key!(),
            value: 0.0,
        }
    }

    pub fn get_value(&self) -> f32 {
        self.value
    }

    pub fn set_value(&mut self, new_value: f32) -> Stage {
        self.value = new_value;
        Stage::Draw
    }

    pub fn view(&self) -> Node<T> {
        let lens = self.lens;

        ui!("slider" [
            .key(self.key)
            .on_draw(true,
                move |t: &T, _ctx: &mut DrawCtx| {
                    let _this = lens.get(t);
                    _ctx.canvas.save();
            })
        ])
    }
}

// ---------- TextBox ----------
#[derive(Debug)]
pub struct TextBox<T: 'static> {
    lens: &'static dyn Lens<In = T, Out = Self>,
    key: Key,
    text: String,
}

impl<T> TextBox<T> {
    #[track_caller]
    pub fn new(lens: impl Lens<In = T, Out = Self>) -> Self {
        Self {
            lens: lens.leak(),
            key: new_key!(),
            text: String::new(),
        }
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, new_text: &str) -> Stage {
        self.text.clear();
        self.text.push_str(new_text);
        // TODO - clear cursor pos, etc.
        Stage::Layout
    }

    pub fn clicked(&mut self, _app: &mut App<T>) -> Stage {
        Stage::Build
    }

    pub fn view(&self) -> Node<T> {
        ui!("text-box" [
            .key(self.key)
            .style_on_draw(move |_: &T, s: &mut Style| s.min_height = s.min_height.max(s.font_size))
        ])
    }
}

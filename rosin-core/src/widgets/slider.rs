#![forbid(unsafe_code)]

use std::{cell::Cell, fmt::Debug, rc::Rc};

use druid_shell::{
    kurbo::{Circle, Line, Point},
    piet::{Color, RenderContext},
};

use crate::prelude::*;

// ---------- Slider ----------
#[derive(Debug)]
pub struct Slider {
    pub key: Key,
    data: Rc<Data>,
}

#[derive(Debug)]
struct Data {
    value: Cell<f64>,
    horizontal: Cell<bool>,
    changed: Cell<bool>,
}

impl Slider {
    pub fn new(value: f64, horizontal: bool) -> Self {
        Self {
            key: Key::new(),
            data: Rc::new(Data {
                value: Cell::new(value),
                horizontal: Cell::new(horizontal),
                changed: Cell::new(false),
            }),
        }
    }

    pub fn set(&self, new_value: f64) -> Phase {
        self.data.value.replace(new_value);
        self.data.changed.replace(true);
        Phase::Draw
    }

    pub fn get(&self) -> f64 {
        self.data.value.get()
    }

    pub fn view<S, H>(&self) -> Node<S, H> {
        let weak1 = Rc::downgrade(&self.data);
        let weak2 = Rc::downgrade(&self.data);
        let weak3 = Rc::downgrade(&self.data);

        ui!([
            .key(self.key)
            .event(On::PointerDown, move |_, ctx: &mut EventCtx<S, H>| {
                let this = if let Some(this) = weak1.upgrade() { this } else { return Some(Phase::Idle) };
                let info = ctx.pointer()?;

                if info.button.is_left() {
                    if this.horizontal.get() {
                        this.value.set((info.pos_x / ctx.width()).into());
                    } else {
                        this.value.set((info.pos_y / ctx.height()).into());
                    }

                    ctx.emit_change();
                    Some(Phase::Draw)
                } else {
                    Some(Phase::Idle)
                }
            })
            .event(On::PointerMove, move |_, ctx: &mut EventCtx<S, H>| {
                let this = if let Some(this) = weak2.upgrade() { this } else { return Some(Phase::Idle) };
                let info = ctx.pointer()?;

                if info.buttons.has_left() {
                    if this.horizontal.get() {
                        this.value.set((info.pos_x / ctx.width()).into());
                    } else {
                        this.value.set((info.pos_y / ctx.height()).into());
                    }

                    ctx.emit_change();
                    Some(Phase::Draw)
                } else {
                    Some(Phase::Idle)
                }
            })
            .on_draw(true, move |_, ctx: &mut DrawCtx| {
                // If the underlying data is gone, then just return since there's nothing to draw.
                // TODO: Maybe log something?
                //       Could also draw the cache, if available.
                let this = if let Some(this) = weak3.upgrade() { this } else { return };
                if !this.changed.get() && !ctx.must_draw { return }
                this.changed.set(false);

                let track = Line::new(
                    Point { x: 0.0, y: ctx.height/2.0},
                    Point { x: ctx.width as f64, y: ctx.height/2.0},
                );

                ctx.piet.stroke(track, &Color::BLACK, 5.0);

                let control = Circle::new(Point { x: this.value.get() * ctx.width, y: ctx.height/2.0 }, 10.0);

                ctx.piet.fill(control, &Color::BLACK);
            })
        ])
    }
}

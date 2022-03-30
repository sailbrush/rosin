#![forbid(unsafe_code)]

use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    rc::Rc,
    sync::Arc,
};

use druid_shell::{
    piet::{Color, FontFamily, RenderContext, Text, TextLayoutBuilder},
    KbKey,
};

use crate::prelude::*;

// ---------- Text Box ----------
#[derive(Debug)]
pub struct TextBox {
    pub key: Key,
    data: Rc<Data>,
}

#[derive(Debug)]
struct Data {
    text: RefCell<String>,
    changed: Cell<bool>,
}

impl TextBox {
    pub fn new(text: &str) -> Self {
        Self {
            key: Key::new(),
            data: Rc::new(Data {
                text: RefCell::new(text.to_owned()),
                changed: Cell::new(false),
            }),
        }
    }

    pub fn set_text(&mut self, new_text: &str) -> Phase {
        let mut text = self.data.text.borrow_mut();
        text.clear();
        text.push_str(new_text);
        self.data.changed.replace(true);
        Phase::Draw
    }

    pub fn append_text(&mut self, new_text: &str) -> Phase {
        let mut text = self.data.text.borrow_mut();
        text.push_str(new_text);
        self.data.changed.replace(true);
        Phase::Draw
    }

    pub fn view<S, H>(&self) -> Node<S, H> {
        let key = self.key;
        let weak1 = Rc::downgrade(&self.data);
        let weak2 = Rc::downgrade(&self.data);

        ui!([
            .key(key)
            .event(On::MouseDown, move |_, ctx| {
                ctx.focus_on(key);
                Phase::Draw
            })
            .event(On::KeyDown, move |_, ctx: &mut EventCtx<S, H>| {
                if let Some(this) = weak1.upgrade() {
                    ctx.emit_change();
                    let event = ctx.event_info.clone().unwrap_key();
                    match &event.key {
                        KbKey::Character(c) => {
                            this.text.borrow_mut().push_str(c);
                            this.changed.replace(true);
                            return Phase::Draw;
                        },
                        KbKey::Enter => {
                            this.text.borrow_mut().push('\n');
                            this.changed.replace(true);
                            return Phase::Draw;
                        },
                        KbKey::Backspace => {
                            this.text.borrow_mut().pop();
                            this.changed.replace(true);
                            return Phase::Draw;
                        },
                        _ => {},
                    }
                }
                Phase::Idle
            })
            .on_draw(true, move |_, ctx: &mut DrawCtx| {
                // If the underlying data is gone, then just return since there's nothing to draw.
                // TODO: Maybe log something?
                //       Could also draw the cache, if available.
                let this = if let Some(this) = weak2.upgrade() { this } else { return };
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

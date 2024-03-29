#![forbid(unsafe_code)]

use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    rc::Rc,
    sync::Arc,
};

use druid_shell::{
    piet::{FontFamily, RenderContext, Text, TextLayoutBuilder},
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

    pub fn view<S, H>(&self) -> View<S, H> {
        let key = self.key;
        let weak1 = Rc::downgrade(&self.data);
        let weak2 = Rc::downgrade(&self.data);

        ui!([
            .key(key)
            .event(On::PointerDown, move |_, ctx| {
                ctx.focus_on(key);
                Some(Phase::Draw)
            })
            .event(On::Keyboard, move |_, ctx: &mut EventCtx<S, H>| {
                if let Some(this) = weak1.upgrade() {
                    ctx.emit_change();
                    match &ctx.keyboard()?.key {
                        KbKey::Character(c) => {
                            this.text.borrow_mut().push_str(c);
                            this.changed.replace(true);
                            return Some(Phase::Draw);
                        },
                        KbKey::Enter => {
                            this.text.borrow_mut().push('\n');
                            this.changed.replace(true);
                            return Some(Phase::Draw);
                        },
                        KbKey::Backspace => {
                            this.text.borrow_mut().pop();
                            this.changed.replace(true);
                            return Some(Phase::Draw);
                        },
                        _ => {},
                    }
                }
                Some(Phase::Idle)
            })
            .on_draw(true, move |_, ctx: &mut DrawCtx| {
                // If the underlying data is gone, then just return since there's nothing to draw.
                // TODO: Maybe log something?
                //       Could also draw the cache, if available.
                let this = if let Some(this) = weak2.upgrade() { this } else { return };
                if !this.changed.get() && !ctx.must_draw { return }
                this.changed.set(false);

                let font_color = ctx.style.color.clone();

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

#![forbid(unsafe_code)]

use std::fmt::Debug;

use crate::prelude::*;

pub use crate::button;

/*
// ---------- Example ----------
#[derive(Debug)]
pub struct Example<T: 'static> {
    lens: &'static dyn Lens<In = T, Out = Self>,
    key: Key,
}

impl<T> Example<T> {
    #[track_caller]
    pub fn new(lens: impl Lens<In = T, Out = Self>) -> Self {
        Self {
            lens: lens.leak(),
            key: new_key!(),
        }
    }

    pub fn view(&self) -> Node<T> {
        let lens = self.lens;
        let key = self.key;

        ui!("example" [
            .key(key)
            .event(On::MouseDown, move |t: &mut T, _app: &mut App<T>| {
                let this = lens.get_mut(t);
                Stage::Paint
            })
        ])
    }
}
*/

// ---------- Button ----------
#[macro_export]
macro_rules! button {
    ($al:ident, $text:literal, $($callback:tt)*) => {
        ui!($al, [
            .content(Content::Label($text))
            .event(On::MouseDown, $($callback)*)
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
        Stage::Paint
    }

    pub fn view(&self) -> Node<T> {
        let lens = self.lens;
        let key = self.key;

        ui!("slider" [
            .key(key)
            .event(On::MouseDown, move |t: &mut T, app: &mut App<T>| {
                let _this = lens.get_mut(t);
                app.focus_on_ancestor(key);
                app.emit_change();
                Stage::Paint
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
        let lens = self.lens;
        let key = self.key;

        ui!("text-box" [
            .key(key)
            .style_on_draw(move |_: &T, t: &mut Style| t.min_height = t.min_height.max(t.font_size))
            .event(On::MouseDown, move |t: &mut T, app: &mut App<T>| {
                let this = lens.get_mut(t);
                app.focus_on_ancestor(key);
                this.clicked(app)
            })
        ])
    }
}

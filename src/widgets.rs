#![forbid(unsafe_code)]

use crate::prelude::*;

pub use crate::button;

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
pub struct Slider<T> {
    key: Key,
    lens: Box<Lens<T, Self>>,
    pub value: f32,
}

impl<T: 'static> Slider<T> {
    #[track_caller]
    pub fn new(lens: Lens<T, Self>) -> Self {
        Self {
            key: new_key!(),
            lens: Box::new(lens),
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

    pub fn view<'a>(&self, al: &'a Alloc) -> Node<'a, T> {
        let lens = self.lens.clone();
        let key = self.key.clone();

        ui!(al, "slider" [
            .key(key)
            .event(On::MouseDown, al.alloc(move |state: &mut T, app: &mut App<T>| {
                let this = lens.get_mut(state);
                app.focus_on_ancestor(key);
                app.emit_change();
                Stage::Paint
            }))
        ])
    }
}

// ---------- TextBox ----------
pub struct TextBox<T> {
    key: Key,
    lens: Lens<T, Self>,
    text: String,
}

impl<T: 'static> TextBox<T> {
    #[track_caller]
    pub fn new(lens: Lens<T, Self>) -> Self {
        Self {
            key: new_key!(),
            lens,
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

    pub fn view<'a>(&self, al: &'a Alloc) -> Node<'a, T> {
        let lens = self.lens.clone();
        let key = self.key.clone();

        ui!(al, "text-box" [
            .key(key)
            .style_on_draw(&|_, style: &mut Style| style.min_height = style.min_height.max(style.font_size))
            .content(Content::DynamicLabel(al.alloc(move |state: &'a T| {
                lens.get(state).text.as_str()
            })))
            .event(On::MouseDown, al.alloc(move |state: &mut T, app: &mut App<T>| {
                let this = lens.get_mut(state);
                app.focus_on_ancestor(key);
                this.clicked(app)
            }))
        ])
    }

    pub fn clicked(&mut self, _app: &mut App<T>) -> Stage {
        Stage::Build
    }
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::prelude::*;
use rosin::widgets::*;

//#[derive(Debug)]
pub struct State {
    text_box: TextBox<Self>,
    other: String,
}

impl State {
    pub fn display(&self) -> &str {
        self.other.as_str()
    }
}

pub fn main_view<'a>(_state: &State, al: &'a Alloc) -> Node<'a, State> {
    ui!(al, "root" [
        "display" []
        "row" [
            "btn" []
            "btn" []
            "btn" []
        ]
        "row" [
            "btn" []
            "btn" []
            "btn" []
        ]
        "row" [
            "btn" []
            "btn" []
            "btn" []
        ]
    ])
}

fn main() {
    let state = State {
        text_box: TextBox::new(new_lens!(State.text_box)),
        other: String::new(),
    };

    let view = new_view!(main_view);
    let style = new_style!("/examples/min.css");
    let window = WindowDesc::new(view).with_title("Rosin Window").with_size(250.0, 250.0);

    App::new()
        .use_style(style)
        .add_window(window)
        .run(state)
        .expect("Failed to launch");
}

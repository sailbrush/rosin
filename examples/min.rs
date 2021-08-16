#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::prelude::*;
use rosin::widgets::*;

#[derive(Debug)]
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
            "btn" []
            "btn" []

        ]

        "row" [
            "btn" []
            "btn" []
            "btn" []
            "btn" []
            "btn" []
        ]
    ])
}

fn main() {
    let state = State {
        text_box: TextBox::new(lens!(State => text_box)),
        other: String::new(),
    };

    let view = new_view!(main_view);
    let stylesheet = new_style!("examples/min.css");
    let window = WindowDesc::new(view).with_title("Rosin Window").with_size(650.0, 650.0);

    AppLauncher::default()
        .use_style(stylesheet)
        .add_window(window)
        .add_font_bytes(0, include_bytes!("fonts/Roboto-Regular.ttf"))
        .run(state)
        .expect("Failed to launch");
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::prelude::*;
use rosin::widgets::*;

#[derive(Debug)]
pub struct State {
    var: Vec<&'static str>,
    height: f32,
}

pub fn main_view<'a>(alloc: &'a Alloc, state: &State) -> UI<'a, State> {
    ui! { alloc,
        "root" [
            "left" []
            "right" []
        ]
    }
}

fn main() {
    let state = State {
        var: Vec::new(),
        height: 200.0,
    };

    let view = view_new!(main_view);
    let style = style_new!("/examples/min.css");
    let window = WindowDesc::new(view).with_title("Rosin Window").with_size(500.0, 500.0);

    App::new()
        .add_window(window)
        .use_style(style)
        .run(state)
        .expect("Failed to launch");
}

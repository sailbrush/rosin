#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::prelude::*;
use rosin::widgets::*;

pub struct State {
    display: DynLabel,
}

pub fn main_view(state: &State) -> Node<State> {
    ui!(None, "root" [
        (state.display.view(lens!(State => display)))
    ])
}

fn main() {
    let state = State {
        display: DynLabel::new("Hello World!"),
    };

    let view = new_view!(main_view);
    let stylesheet = new_style!("examples/min.css");
    let window = WindowDesc::new(view).with_title("Rosin Window").with_size(500.0, 500.0);

    AppLauncher::default()
        .use_style(stylesheet)
        .add_window(window)
        .add_font_bytes(0, include_bytes!("fonts/Roboto-Regular.ttf"))
        .run(state)
        .expect("Failed to launch");
}

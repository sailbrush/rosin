#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use rosin::prelude::*;
use rosin::widgets::*;

pub struct State {
    style: Arc<Stylesheet>,
    text: Slider,
}

pub fn main_view(state: &State) -> Node<State, WindowHandle> {
    ui!(state.style.clone(), "root"["text"(state.text.view())])
}

#[rustfmt::skip]
fn main() {
    let view = new_view!(main_view);

    let window = WindowDesc::new(view)
        .with_title("Rosin Window")
        .with_size(500.0, 500.0);

    let mut rl = ResourceLoader::default();

    let state = State {
        style: load_css!(rl, "examples/min.css"),
        text: Slider::new(0.0, true),
    };

    AppLauncher::new(rl, window)
        .run(state)
        .expect("Failed to launch");
}

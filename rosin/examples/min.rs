#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use druid_shell::WindowHandle;
use rosin::prelude::*;
use rosin::widgets::*;

pub struct State {
    display: DynLabel,
}

pub fn main_view(state: &State) -> Node<State, WindowHandle> {
    ui!("root"[(state.display.view())])
}

#[rustfmt::skip]
fn main() {
    let view = new_view!(main_view);

    let window = WindowDesc::new(view)
        .with_title("Rosin Window")
        .with_size(500.0, 500.0);

    let mut rl = ResourceLoader::new();

    load_css!(rl, "examples/min.css");

    let state = State {
        display: DynLabel::new("Hello World!"),
    };

    AppLauncher::new(rl, window)
        .run(state)
        .expect("Failed to launch");
}

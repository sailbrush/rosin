#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::prelude::*;
use rosin::widgets::*;

pub struct State {
    style: Stylesheet,
    label: DynLabel,
    count: u32,
}

pub fn main_view(state: &State) -> View<State, WindowHandle> {
    ui!(state.style.clone(), "root" [
        "text" (state.label.view())
        "bump" (button("+", |s: &mut State, _ctx| {
            s.count += 1;
            let phase = s.label.set_text(&s.count.to_string());
            Some(phase)
        }))
    ])
}

#[rustfmt::skip]
fn main() {
    let view = new_viewfn!(main_view);

    let window = WindowDesc::new(view)
        .with_title("Rosin Window")
        .with_size(500.0, 500.0);

    let mut rl = ResourceLoader::default();

    let state = State {
        style: load_css!(rl, "examples/min.css"),
        label: DynLabel::new("0"),
        count: 0,
    };

    AppLauncher::new(rl, window)
        .run(state)
        .expect("Failed to launch");
}

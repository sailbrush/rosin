#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::{prelude::*, widgets::*};

struct State {
    style: Stylesheet,
    count: Var<i32>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            style: stylesheet!("examples/styles/counter.css"),
            count: Var::new(0),
        }
    }
}

fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    ui.node().id(id!()).style_sheet(&state.style).classes("root").children(|ui| {
        label(ui, id!(), *state.count).classes("number");
        button(ui, id!(), "Count", |s, _| {
            *s.count.write() += 1;
        });
    });
}

#[rustfmt::skip]
fn main() {
    env_logger::init();

    let window = WindowDesc::new(callback!(main_view))
        .title("Counter Example")
        .size(400, 300)
        .min_size(250, 150);

    AppLauncher::new(window)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}

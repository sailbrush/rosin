use rosin::{prelude::*, text, widgets::*};

struct State {
    style: Stylesheet,
}
impl State {
    fn default() -> Self {
        Self {
            style: stylesheet!("examples/styles/two_window.css"),
        }
    }
}

fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    ui.node().id(id!()).style_sheet(&state.style).classes("root").children(|ui| {
        label(ui, id!(), "Window 1");
    });
}

fn main_view2(state: &State, ui: &mut Ui<State, WindowHandle>) {
    ui.node().id(id!()).style_sheet(&state.style).classes("root").children(|ui| {
        label(ui, id!(), "Window 2");
    });
}

#[rustfmt::skip]
fn main() {
    env_logger::init();

    let window1 = WindowDesc::new(callback!(main_view))
        .title("Two Window Test")
        .min_size(800, 650)
        .size(800, 650);

    let window2 = WindowDesc::new(callback!(main_view2))
        .title("Two Window Test")
        .min_size(800, 400)
        .size(800, 400);

    AppLauncher::new(window1).add_window(window2)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}

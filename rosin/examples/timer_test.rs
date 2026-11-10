use rosin::{prelude::*, text, widgets::*};

struct State {
}
impl State {
    fn default() -> Self {
        Self {
        }
    }
}

fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    ui.node().id(id!()).classes("root").children(|ui| {
        button(ui, id!(), "Set Alarm", move |_, _| {
            
        } );
    }).event(On::Timer, |s, ctx| {
    });
}


#[rustfmt::skip]
fn main() {
    env_logger::init();

    let window1 = WindowDesc::new(callback!(main_view))
        .title("Two Window Test")
        .min_size(800, 650)
        .size(800, 650);

    AppLauncher::new(window1)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}

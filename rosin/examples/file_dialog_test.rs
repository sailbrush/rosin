#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::{keyboard_types::NamedKey, prelude::*, widgets::*};

struct State {
    location: Var<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            location: Var::new("/".into()),
        }
    }
}

impl State {
    fn go(&self, ctx: &EventCtx<'_, WindowHandle>) {
        ctx.platform().open_file_dialog(ctx.id(), FileDialogOptions::new());
    }
    fn go2(&self, ctx: &EventCtx<'_, WindowHandle>) {
        ctx.platform().save_file_dialog(ctx.id(), FileDialogOptions::new());
    }
}

fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    ui.node()
        .id(id!())
        .style_sheet(dark_theme())
        .classes("root").children(|ui| {
            button(ui, id!(), "Open Dialog", |s, ctx| s.go(ctx));
            button(ui, id!(), "Save Dialog", |s, ctx| s.go2(ctx));
            label(ui, id!(), ui_format!("{:?}", state.location));
        });
}

#[rustfmt::skip]
fn main() {
    env_logger::init();

    let window = WindowDesc::new(callback!(main_view))
        .title("URL Example")
        .size(400, 150)
        .min_size(250, 150)
        .max_size(600, 150);

    AppLauncher::new(window)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::{prelude::*, widgets::*};
use std::fs::File;
use std::io::Read;
struct State {
    location: Var<String>,
    edit_box: TextBox,
    edit_text: Var<String>,
    style: Stylesheet
}

impl Default for State {
    fn default() -> Self {
        Self {
            location: Var::new("/".into()),
            edit_box: TextBox::default(),
            edit_text: Var::new("".into()),
            style: stylesheet!("examples/styles/text_editor.css"),
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

    fn open(&mut self) {
        let file = File::open(self.location.get());
        let mut content = String::new();
        file.unwrap().read_to_string(&mut content);
        self.edit_text.set(content);
    }
}

fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    println!("{:?}", state.location);
    ui.node().id(id!()).style_sheet(&state.style).classes("root").children(|ui| {
        button(ui, id!(), "Open Dialog", |s, ctx| s.go(ctx)).event(On::FileDialog, |s, ctx| {
            match ctx.info() {
                EventInfo::File(file) => match file {
                    FileDialogResponse::Opened(files) => {
                        s.location.set(files[0].clone().into_os_string().into_string().unwrap().into());
                        s.open();
                    }
                    _ => {}
                },
                _ => {}
            }
        });
        button(ui, id!(), "Save Dialog", |s, ctx| s.go2(ctx)).event(On::FileDialog, |s, ctx| {
            match ctx.info() {
                EventInfo::File(file) => match file {
                    FileDialogResponse::Saved(file) => {
                        s.location.set(file.clone().into_os_string().into_string().unwrap().into());
                    }
                    _ => {}
                },
                _ => {}
            }
        });
        state.edit_box.view(ui, id!(), *state.edit_text);
    });
}

#[rustfmt::skip]
fn main() {
    env_logger::init();

    let window = WindowDesc::new(callback!(main_view))
        .title("Text Editor")
        .size(400, 400)
        .min_size(400, 400)
        .max_size(400, 400);

    AppLauncher::new(window)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
extern crate rosin;
use rosin::prelude::*;

extern crate calculator;
use calculator::*;

fn main() {
    let store = Store::new();
    let mut app = App::new();
    let style = style_new!("/src/style.css");
    let view = view_new!(main_view);

    app.set_style(style);
    app.create_window(WindowBuilder::default().with_view(view)).unwrap();
    app.run(store);
}

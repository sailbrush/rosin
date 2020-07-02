#![feature(trace_macros)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
extern crate rosin;
use rosin::prelude::*;
use rosin::widgets::*;

#[derive(Debug)]
pub struct State {
    var: Vec<&'static str>,
}


//trace_macros!(true);

pub fn main_view<'a>(alloc: &'a Bump, state: &State) -> UI<'a, State> {
    ui! { alloc;
        "root" => []
    }
}

fn main() {
    let mut store = State { var: Vec::new() };

    let view = view_new!(main_view);
    let style = style_new!("/examples/min.css");
    let window = WindowDesc::new(view).with_title("Rosin Window").with_size(500.0, 500.0);

    AppLauncher::new()
        .add_window(window)
        .use_style(style)
        .launch(store)
        .expect("Failed to launch");
}

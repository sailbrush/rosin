#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::{prelude::*, vello::peniko::Color, widgets::*};

struct State {
    style: Stylesheet,

    celsius_value: Var<String>,
    celsius_textbox: TextBox,

    fahrenheit_value: Var<String>,
    fahrenheit_textbox: TextBox,
}

impl Default for State {
    fn default() -> Self {
        Self {
            style: stylesheet!("examples/styles/7gui.css"),

            celsius_value: Var::new("-40".into()),
            celsius_textbox: TextBox::default(),

            fahrenheit_value: Var::new("-40".into()),
            fahrenheit_textbox: TextBox::default(),
        }
    }
}

impl State {
    fn celsius_changed(&mut self, _: &mut EventCtx<WindowHandle>) {
        // When this text changes, update the other value
        if let Ok(c) = self.celsius_value.read().parse::<f32>() {
            let converted = (c * 9.0 / 5.0) + 32.0;
            *self.fahrenheit_value.write() = format!("{:.2}", converted);
        }
    }

    fn celsius_style(&self, style: &mut Style) {
        let f = self.fahrenheit_value.get();
        if !f.is_empty() && f.parse::<f32>().is_err() {
            style.background_color = Color::from_rgb8(255, 0, 0);
        }
    }

    fn fahrenheit_changed(&mut self, _: &mut EventCtx<WindowHandle>) {
        // When this text changes, update the other value
        if let Ok(f) = self.fahrenheit_value.read().parse::<f32>() {
            let converted = (f - 32.0) * 5.0 / 9.0;
            *self.celsius_value.write() = format!("{:.2}", converted);
        }
    }

    fn fahrenheit_style(&self, style: &mut Style) {
        let c = self.celsius_value.get();
        if !c.is_empty() && c.parse::<f32>().is_err() {
            style.background_color = Color::from_rgb8(255, 0, 0);
        }
    }
}

fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    // Create the root node, attach a stylesheet, assign a CSS class, and add children
    ui.node().style_sheet(&state.style).classes("root").children(|ui| {
        // Build the first textbox, add a change event handler, and a style callback
        state
            .celsius_textbox
            .view(ui, id!(), *state.celsius_value)
            .event(On::Change, State::celsius_changed)
            .on_style(State::celsius_style);

        // Build a simple label
        label(ui, id!(), "Celsius = ");

        // Build the second textbox, add a change event handler, and a style callback
        state
            .fahrenheit_textbox
            .view(ui, id!(), *state.fahrenheit_value)
            .event(On::Change, State::fahrenheit_changed)
            .on_style(State::fahrenheit_style);

        // Build another label
        label(ui, id!(), "Fahrenheit");
    });
}

#[rustfmt::skip]
fn main() {
    env_logger::init();

    let window = WindowDesc::new(callback!(main_view))
        .title("7GUI Temperature")
        .size(500, 100)
        .min_size(500, 100);

    AppLauncher::new(window)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}

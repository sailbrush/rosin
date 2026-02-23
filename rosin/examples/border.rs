#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::{prelude::*, widgets::*};

struct State {
    style: Stylesheet,
    top_width: Var<f64>,
    right_width: Var<f64>,
    bottom_width: Var<f64>,
    left_width: Var<f64>,
    top_left_radius: Var<f64>,
    top_right_radius: Var<f64>,
    bottom_right_radius: Var<f64>,
    bottom_left_radius: Var<f64>,
    outline: Var<f64>,
    opacity: Var<f64>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            style: stylesheet!("examples/styles/border.css"),
            top_width: Var::new(20.0),
            right_width: Var::new(20.0),
            bottom_width: Var::new(20.0),
            left_width: Var::new(20.0),
            top_left_radius: Var::new(20.0),
            top_right_radius: Var::new(20.0),
            bottom_right_radius: Var::new(20.0),
            bottom_left_radius: Var::new(20.0),
            outline: Var::new(0.0),
            opacity: Var::new(1.0),
        }
    }
}

fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    // Create the root node and set the stylesheet
    ui.node()
        .classes("root")
        .style_sheet(&state.style)
        // Add child nodes to the root
        .children(|ui| {
            // Add a child with the CSS class "left"
            ui.node().id(id!()).classes("left").children(|ui| {
                // Add labels and sliders as children to the left node
                label(ui, id!(), "top-width:");
                SliderParams::new().max(200.0).view(ui, id!(), *state.top_width);

                label(ui, id!(), "right-width:");
                SliderParams::new().max(200.0).view(ui, id!(), *state.right_width);

                label(ui, id!(), "bottom-width:");
                SliderParams::new().max(200.0).view(ui, id!(), *state.bottom_width);

                label(ui, id!(), "left-width:");
                SliderParams::new().max(200.0).view(ui, id!(), *state.left_width);

                label(ui, id!(), "top-left-radius:");
                SliderParams::new().max(200.0).view(ui, id!(), *state.top_left_radius);

                label(ui, id!(), "top-right-radius:");
                SliderParams::new().max(200.0).view(ui, id!(), *state.top_right_radius);

                label(ui, id!(), "bottom-right-radius:");
                SliderParams::new().max(200.0).view(ui, id!(), *state.bottom_right_radius);

                label(ui, id!(), "bottom-left-radius:");
                SliderParams::new().max(200.0).view(ui, id!(), *state.bottom_left_radius);

                label(ui, id!(), "outline:");
                SliderParams::new().max(50.0).view(ui, id!(), *state.outline);

                label(ui, id!(), "opacity:");
                SliderParams::new().view(ui, id!(), *state.opacity);
            });
            // Add another child to the root node, this time with the CSS class "right"
            ui.node().classes("right").children(move |ui| {
                // Add a single child node with the CSS class "box"
                ui.node()
                    .classes("box")
                    // Set a callback to change this node's style before drawing
                    // This will automatically be re-run when any of the Var<T>'s that were read are later changed.
                    .on_style(move |s, style| {
                        style.border_top_width = s.top_width.get().into();
                        style.border_right_width = s.right_width.get().into();
                        style.border_bottom_width = s.bottom_width.get().into();
                        style.border_left_width = s.left_width.get().into();
                        style.border_top_left_radius = s.top_left_radius.get().into();
                        style.border_top_right_radius = s.top_right_radius.get().into();
                        style.border_bottom_right_radius = s.bottom_right_radius.get().into();
                        style.border_bottom_left_radius = s.bottom_left_radius.get().into();
                        style.outline_width = s.outline.get().into();
                        style.opacity = s.opacity.get() as f32;
                    });
            });
        });
}

#[rustfmt::skip]
fn main() {
    env_logger::init();

    let window = WindowDesc::new(callback!(main_view))
        .title("CSS Border Example")
        .min_size(800, 650)
        .size(800, 650);

    AppLauncher::new(window)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}

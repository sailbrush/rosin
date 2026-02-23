#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::prelude::*;

const SHORT_TEXT: &str = "Sphinx of black quartz, judge my vow.";
const LONG_TEXT: &str = "We choose to go to the Moon in this decade and do the other things, \
not because they are easy, but because they are hard; because that goal will serve to organize \
and measure the best of our energies and skills, because that challenge is one that we are willing \
to accept, one we are unwilling to postpone...";

struct State {
    style: Stylesheet,
}

impl Default for State {
    fn default() -> Self {
        Self {
            style: stylesheet!("examples/styles/text_layout.css"),
        }
    }
}

fn pair_row(ui: &mut Ui<State, WindowHandle>, title: &'static str, classes: &'static str, s: &'static str) {
    ui.node().classes("test-row").children(|ui| {
        ui.node().classes("label").text(title);

        ui.node().classes("pair").children(|ui| {
            // Render the text by attaching it to a node, and by calling the convenience function in on_canvas.
            // Both should render the same.
            ui.node().classes("cell").classes(classes).text(s);
            ui.node().classes("cell").classes(classes).on_canvas(move |_, ctx| {
                ctx.draw_text(s);
            });
        });
    });
}

fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    ui.node().style_sheet(&state.style).classes("root").children(|ui| {
        ui.node().classes("pane").children(|ui| {
            ui.node().classes("test-grid").children(|ui| {
                ui.node().classes("test-column").children(|ui| {
                    pair_row(ui, "default", "", LONG_TEXT);
                    pair_row(ui, "percent padding", "padding-left-percent", LONG_TEXT);
                    pair_row(ui, "stretch padding", "padding-stretch", SHORT_TEXT);
                    pair_row(ui, "align left", "align-left", LONG_TEXT);
                    pair_row(ui, "align center", "align-center", LONG_TEXT);
                    pair_row(ui, "align right", "align-right", LONG_TEXT);
                    pair_row(ui, "justify", "align-justify", LONG_TEXT);
                });
                ui.node().classes("test-column").children(|ui| {
                    pair_row(ui, "padding and border", "padding-and-border", LONG_TEXT);
                    pair_row(ui, "line-height 250%", "line-height-percent", LONG_TEXT);
                    pair_row(ui, "line-height 20px", "line-height-px", LONG_TEXT);
                    pair_row(ui, "letter spacing", "letter-spacing", LONG_TEXT);
                    pair_row(ui, "word spacing", "word-spacing", LONG_TEXT);
                    pair_row(ui, "weight 900", "font-weight-900", LONG_TEXT);
                    pair_row(ui, "italic", "font-style-italic", LONG_TEXT);
                });
            });
        });
    });
}

#[rustfmt::skip]
fn main() {
    env_logger::init();

    let window = WindowDesc::new(callback!(main_view))
        .title("Text Layout Example")
        .size(1000, 1000);

    AppLauncher::new(window)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}

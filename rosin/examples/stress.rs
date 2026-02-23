#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::{prelude::*, vello::peniko::Color, widgets::*};

struct State {
    style: Stylesheet,
    perf: PerfDisplay,
}

impl Default for State {
    fn default() -> Self {
        Self {
            style: stylesheet!("examples/styles/stress.css"),
            perf: PerfDisplay::default(),
        }
    }
}

// Create 11,112 nodes that need their layout recalculated on resize.
// Comparable to the yoga 'huge nested 10k' benchmark.
// Gets about 130 fps on an M1 MacBook Air in release mode.
fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    ui.node().style_sheet(&state.style).classes("root").children(|ui| {
        state.perf.view(ui, id!());

        for a in 0u64..10 {
            ui.node().classes("node").children(|ui| {
                for b in 0u64..10 {
                    ui.node().classes("node row").children(|ui| {
                        for c in 0u64..10 {
                            ui.node().classes("node").children(|ui| {
                                for d in 0u64..10 {
                                    let i = a * 1000 + b * 100 + c * 10 + d;

                                    let r = ((i.wrapping_mul(37)) & 0xFF) as u8;
                                    let g = ((i.wrapping_mul(57)) & 0xFF) as u8;
                                    let b = ((i.wrapping_mul(97)) & 0xFF) as u8;

                                    ui.node().classes("node row").on_style(move |_, s| {
                                        s.background_color = Color::from_rgb8(r, g, b);
                                    });
                                }
                            });
                        }
                    });
                }
            });
        }
    });
}

#[rustfmt::skip]
#[allow(unreachable_code)]
fn main() {
    #[cfg(debug_assertions)]
    panic!("\n⚠️  This example should be run in release mode.\n");

    env_logger::init();

    let window = WindowDesc::new(callback!(main_view))
        .title("Layout Stress Test")
        .size(1000, 1000)
        .min_size(250, 250);

    AppLauncher::new(window)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}

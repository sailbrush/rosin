#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use druid_shell::kurbo::{Line, Point};
use druid_shell::piet::{Color, RenderContext};
use druid_shell::KbKey;
use rosin::prelude::*;
use rosin::widgets::*;

pub struct State {
    style: Stylesheet,
    lines: Vec<Vec<Point>>,
}

pub fn main_view(state: &State) -> Node<State, WindowHandle> {
    ui!(state.style.clone(), "root" [{
            .event(On::PointerDown, |s: &mut State, _| {
                s.lines.push(Vec::new());
                Phase::Build
            })
            .event(On::Keyboard, |s: &mut State, ctx| {
                let event = ctx.event_info.unwrap_key();
                if event.key == KbKey::Backspace {
                    s.lines.pop();
                }
                Phase::Build
            })
            .event(On::PointerMove, |s: &mut State, ctx| {
                let event = ctx.event_info.unwrap_pointer();
                if event.buttons.has_left() {
                    if let Some(line) = s.lines.last_mut() {
                        line.push(Point { x: event.pos_x.into(), y: event.pos_y.into() });
                    }
                }
                Phase::Draw
            })
            .on_draw(false, |s, ctx| {
                for line in &s.lines {
                    let mut prev_point = None;
                    for point in line {
                        if let Some(prev) = prev_point {
                            let path = Line::new(prev, point.clone());
                            ctx.piet.stroke(path, &Color::BLACK, 5.0);
                        }
                        prev_point = Some(point.clone());
                    }
                }
            })
        }

        if state.lines.len() > 0 {
            "clear" (button("Clear", |s: &mut State, _| {
                s.lines = Vec::new();
                Phase::Draw
            }))
        }
    ])
}

#[rustfmt::skip]
fn main() {
    let view = new_view!(main_view);

    let window = WindowDesc::new(view)
        .with_title("Draw")
        .with_size(500.0, 500.0);

    let mut rl = ResourceLoader::default();

    let state = State {
        style: load_css!(rl, "examples/draw.css"),
        lines: Vec::new(),
    };

    AppLauncher::new(rl, window)
        .run(state)
        .expect("Failed to launch");
}

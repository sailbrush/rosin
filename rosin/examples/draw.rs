#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use druid_shell::kurbo::{Line, Point};
use druid_shell::piet::{Color, RenderContext};
use rosin::prelude::*;
use rosin::widgets::*;

pub struct State {
    down: bool,
    lines: Vec<Vec<Point>>,
}

pub fn main_view(_: &State) -> Node<State, WindowHandle> {
    ui!("root" [{
            .event(On::MouseDown, |s: &mut State, _| {
                s.lines.push(Vec::new());
                s.down = true;
                Phase::Draw
            })
            .event(On::MouseUp, |s: &mut State, _| {
                s.down = false;
                Phase::Draw
            })
            .event(On::MouseMove, |s: &mut State, ctx| {
                if s.down {
                    if let EventInfo::Mouse(e) = &mut ctx.event_info {
                        if let Some(line) = s.lines.last_mut() {
                            line.push(e.pos);
                        }
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

        "clear" (button("Clear", |s: &mut State, _| {
            s.lines = Vec::new();
            Phase::Draw
        }))
    ])
}

#[rustfmt::skip]
fn main() {
    let view = new_view!(main_view);

    let window = WindowDesc::new(view)
        .with_title("Draw")
        .with_size(500.0, 500.0);

    let mut rl = ResourceLoader::new();

    load_css!(rl, "examples/draw.css");

    let state = State {
        down: false,
        lines: Vec::new(),
    };

    AppLauncher::new(rl, window)
        .run(state)
        .expect("Failed to launch");
}

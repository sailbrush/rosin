#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use druid_shell::kurbo::{Line, Point};
use druid_shell::piet::{Color, RenderContext};
use rosin::prelude::*;

pub struct State {
    points: Vec<Point>,
}

pub fn main_view(_: &State) -> Node<State, WindowHandle> {
    ui!("root" [
        .event(On::MouseMove, |s: &mut State, ctx| {
            if let EventInfo::Mouse(e) = &mut ctx.event_info {
                s.points.push(e.pos);
            }
            Phase::Draw
        })
        .on_draw(false, |s, ctx| {
            let mut prev_point = None;
            for point in &s.points {
                if let Some(prev) = prev_point {
                    let path = Line::new(prev, point.clone());
                    ctx.piet.stroke(path, &Color::BLACK, 1.0);
                }
                prev_point = Some(point.clone());
            }
        })
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
        points: Vec::new(),
    };

    AppLauncher::new(rl, window)
        .run(state)
        .expect("Failed to launch");
}

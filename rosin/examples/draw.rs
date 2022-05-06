#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use druid_shell::kurbo::{Line, Point};
use druid_shell::piet::{Color, RenderContext};
use druid_shell::KbKey;
use rosin::prelude::*;
use rosin::widgets::*;

pub struct State {
    style: Stylesheet,
    size_control: Slider,
    canvas: Canvas,
}

pub struct Canvas {
    lines: Vec<(f64, Vec<Point>)>,
    brush_size: f64,
}

impl Canvas {
    pub fn new() -> Self {
        Canvas {
            lines: Vec::new(),
            brush_size: 10.0,
        }
    }

    pub fn clear(&mut self) -> Phase {
        self.lines.clear();
        Phase::Draw
    }

    pub fn undo(&mut self) -> Phase {
        self.lines.pop();
        Phase::Draw
    }

    pub fn view(&self) -> Node<State, WindowHandle> {
        ui!("canvas" [{
            .event(On::PointerDown, |s: &mut State, _| {
                s.canvas.lines.push((s.canvas.brush_size, Vec::new()));
                Some(Phase::Build)
            })
            .event(On::PointerMove, |s: &mut State, ctx| {
                if ctx.pointer()?.button.is_left() {
                    if let Some((_, line)) = s.canvas.lines.last_mut() {
                        line.push(Point { x: ctx.pointer()?.pos_x.into(), y: ctx.pointer()?.pos_y.into() });
                    }
                }
                Some(Phase::Draw)
            })
            .on_draw(false, |s, ctx| {
                for (size, line) in &s.canvas.lines {
                    let mut prev_point = None;
                    for point in line {
                        if let Some(prev) = prev_point {
                            let path = Line::new(prev, point.clone());
                            ctx.piet.stroke(path, &Color::BLACK, *size);
                        }
                        prev_point = Some(point.clone());
                    }
                }
            })
        }])
    }
}

pub fn main_view(state: &State) -> Node<State, WindowHandle> {
    ui!(state.style.clone(), "root" [{
            // When the user hits Backspace, call `undo()` on the canvas
            .event(On::Keyboard, |s: &mut State, ctx| {
                if ctx.keyboard()?.key == KbKey::Backspace {
                    Some(s.canvas.undo())
                } else {
                    Some(Phase::Idle)
                }
            })
        }
        "toolbar" [
            "clear" (button("Clear", |s: &mut State, _| {
                Some(s.canvas.clear())
            }))

            // When the slider widget changes, update the canvas's brush size
            (state.size_control.view()
                .event(On::Change, |s: &mut State, _| {
                    s.canvas.brush_size = s.size_control.get() * 50.0 + 0.5;
                    Some(Phase::Idle)
                })
            )
        ]
        (state.canvas.view())
    ])
}

#[rustfmt::skip]
fn main() {
    let view = new_view!(main_view);

    let window = WindowDesc::new(view)
        .with_title("Rosin Draw Example")
        .with_size(1000.0, 800.0);

    let mut rl = ResourceLoader::default();

    let state = State {
        style: load_css!(rl, "examples/draw.css"),
        size_control: Slider::new(0.2, true),
        canvas: Canvas::new(),
    };

    AppLauncher::new(rl, window)
        .run(state)
        .expect("Failed to launch");
}

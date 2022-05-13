#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use druid_shell::kurbo::{BezPath, PathEl};
use druid_shell::piet::{Color, LineCap, LineJoin, RenderContext, StrokeStyle};
use druid_shell::{KbKey, KeyState};
use rosin::prelude::*;
use rosin::widgets::*;

#[derive(Debug)]
pub struct State {
    style: Stylesheet,
    size_control: Slider,
    canvas: Canvas,
}

#[derive(Debug)]
pub struct Canvas {
    lines: Vec<(f64, BezPath)>,
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

    pub fn view(&self) -> View<State, WindowHandle> {
        ui!("canvas" [{
            .event(On::PointerDown, |s: &mut State, ctx| {
                let mut path = BezPath::new();
                path.move_to((ctx.pointer()?.pos_x as f64, ctx.pointer()?.pos_y as f64));
                s.canvas.lines.push((s.canvas.brush_size, path));
                Some(Phase::Build)
            })
            .event(On::PointerMove, |s, ctx| {
                if ctx.pointer()?.buttons.has_left() {
                    if let Some((_, path)) = s.canvas.lines.last_mut() {
                        match path.elements().last() {
                            Some(PathEl::MoveTo(point)) | Some(PathEl::LineTo(point)) => {
                                let new_point = (ctx.pointer()?.pos_x as f64, ctx.pointer()?.pos_y as f64);
                                if (point.x - new_point.0).abs() >= 5.0 || (point.y - new_point.1).abs() >= 5.0 {
                                    path.line_to(new_point);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Some(Phase::Draw)
            })
            .on_draw(false, |s, ctx| {
                let stroke_style = StrokeStyle::new()
                    .line_join(LineJoin::Round)
                    .line_cap(LineCap::Round);
                for (size, path) in &s.canvas.lines {
                    ctx.piet.stroke_styled(path, &Color::BLACK, *size, &stroke_style);
                }
            })
        }])
    }
}

pub fn main_view(state: &State) -> View<State, WindowHandle> {
    ui!(state.style.clone(), "root" [{
            // When the user presses Backspace, call `undo()` on the canvas
            .event(On::Keyboard, |s: &mut State, ctx| {
                if ctx.keyboard()?.state == KeyState::Down && ctx.keyboard()?.key == KbKey::Backspace {
                    Some(s.canvas.undo())
                } else {
                    None
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
    let view = new_viewfn!(main_view);

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

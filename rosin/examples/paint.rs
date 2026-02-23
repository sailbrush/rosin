#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::prelude::*;
use rosin::widgets::*;

use rosin::kurbo::{Affine, BezPath, PathEl, Stroke};
use rosin::peniko::Color;

pub struct State {
    style: Stylesheet,
    canvas: Canvas,
}

impl Default for State {
    fn default() -> Self {
        Self {
            style: stylesheet!("examples/styles/paint.css"),
            canvas: Canvas::new(),
        }
    }
}

pub struct Canvas {
    lines: Var<Vec<(f64, BezPath)>>,
    pub brush_size: Var<f64>,
}

impl Canvas {
    pub fn new() -> Self {
        Canvas {
            lines: Var::new(Vec::new()),
            brush_size: Var::new(1.0),
        }
    }

    pub fn clear(&mut self) {
        self.lines.write().clear();
    }

    pub fn undo(&mut self) {
        self.lines.write().pop();
    }

    pub fn view<'a>(&self, ui: &'a mut Ui<State, WindowHandle>, id: NodeId) -> &'a mut Ui<State, WindowHandle> {
        ui.node()
            .classes("canvas")
            .id(id)
            .event(On::PointerDown, |s, ctx| {
                let Some(pos) = ctx.local_pointer_pos() else {
                    return;
                };

                ctx.begin_pointer_capture();
                let mut path = BezPath::new();
                path.move_to((pos.x as f64, pos.y as f64));
                s.canvas.lines.write().push((s.canvas.brush_size.get(), path));
            })
            .event(On::PointerMove, |s, ctx| {
                let Some(ev) = ctx.pointer() else {
                    return;
                };
                let Some(pos) = ctx.local_pointer_pos() else {
                    return;
                };

                if ev.buttons.contains(PointerButton::Primary) {
                    if let Some((_, path)) = s.canvas.lines.write().last_mut() {
                        match path.elements_mut().last_mut() {
                            Some(PathEl::MoveTo(point)) | Some(PathEl::LineTo(point)) => {
                                if point.distance(pos) >= 1.0 {
                                    path.line_to(pos);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            })
            .event(On::PointerUp, |_, ctx| {
                ctx.end_pointer_capture();
            })
            .on_canvas(|s, ctx| {
                for (size, path) in s.canvas.lines.read().iter() {
                    ctx.scene.stroke(&Stroke::new(*size), Affine::IDENTITY, &Color::BLACK, None, &path);
                }
            })
    }
}

pub fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    ui.node().classes("root").style_sheet(&state.style).children(|ui| {
        ui.node().classes("toolbar").children(|ui| {
            button(ui, id!(), "Clear", |s, _| s.canvas.clear());
            button(ui, id!(), "Undo", |s, _| s.canvas.undo());
            SliderParams::new().min(1.0).max(50.0).view(ui, id!(), *state.canvas.brush_size);
        });

        state.canvas.view(ui, id!());
    });
}

#[rustfmt::skip]
fn main() {
    env_logger::init();

    let window = WindowDesc::new(callback!(main_view))
        .title("Paint Example")
        .size(1000, 800);

    AppLauncher::new(window)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}

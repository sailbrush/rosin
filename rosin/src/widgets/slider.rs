use crate::{
    kurbo::{Affine, Rect},
    peniko::Fill,
    prelude::*,
    widgets::widget_styles,
};

// TODO - use css transform for thumb so layout pass isn't needed, and to make sliding continuous
#[derive(Copy, Clone)]
pub struct SliderParams {
    min: UIParam<f64>,
    max: UIParam<f64>,
}

impl Default for SliderParams {
    fn default() -> Self {
        Self::new()
    }
}

impl SliderParams {
    pub fn new() -> Self {
        Self {
            min: UIParam::Static(0.0),
            max: UIParam::Static(1.0),
        }
    }

    pub fn min(mut self, value: impl Into<UIParam<f64>>) -> Self {
        self.min = value.into();
        self
    }

    pub fn max(mut self, value: impl Into<UIParam<f64>>) -> Self {
        self.max = value.into();
        self
    }

    pub fn view<'a, S, H>(&self, ui: &'a mut Ui<S, H>, id: NodeId, value: WeakVar<f64>) -> &'a mut Ui<S, H> {
        let min = self.min;
        let max = self.max;

        ui.node()
            .id(id)
            .classes("slider")
            .style_sheet(widget_styles())
            .event(On::PointerDown, move |_, ctx| {
                let min = min.get_or(0.0);
                let max = max.get_or(1.0);
                ctx.set_focus(None);
                let Some(ev) = ctx.pointer() else { return };
                let Some(pos) = ctx.local_pointer_pos() else { return };
                if ev.button.is_primary() {
                    ctx.begin_pointer_capture();
                    ctx.emit_change();
                    let content_rect = ctx.padding_box();
                    let t = ((pos.x - content_rect.x0) / (content_rect.x1 - content_rect.x0)).clamp(0.0, 1.0);
                    value.set(t * (max - min) + min);
                }
            })
            .event(On::PointerUp, move |_, ctx| {
                ctx.end_pointer_capture();
            })
            .event(On::PointerMove, move |_, ctx| {
                let min = min.get_or(0.0);
                let max = max.get_or(1.0);
                let Some(pos) = ctx.local_pointer_pos() else { return };
                if ctx.is_pointer_captured() {
                    ctx.emit_change();
                    let content_rect = ctx.padding_box();
                    let t = ((pos.x - content_rect.x0) / (content_rect.x1 - content_rect.x0)).clamp(0.0, 1.0);
                    value.set(t * (max - min) + min);
                }
            })
            .children(|ui| {
                ui.node().id(id!(id)).classes("track").on_canvas(move |_, ctx| {
                    let min = min.get_or(0.0);
                    let max = max.get_or(1.0);

                    let Some(val) = value.read() else { return };

                    let denom = max - min;
                    let t = if denom.abs() <= f64::EPSILON {
                        0.0
                    } else {
                        ((*val - min) / denom).clamp(0.0, 1.0)
                    };

                    let padding_box = ctx.padding_box();
                    let fill_rect = Rect::new(padding_box.x0, padding_box.y0, padding_box.x0 + padding_box.width() * t, padding_box.y1);

                    ctx.scene.fill(Fill::NonZero, Affine::IDENTITY, ctx.style.color, None, &fill_rect);
                });
                ui.node().id(id!(id)).classes("thumb").on_style(move |_, style| {
                    let min = min.get_or(0.0);
                    let max = max.get_or(1.0);

                    let Some(val) = value.read() else { return };
                    style.left = Unit::Stretch(val.clamp(min, max) as f32);
                    style.right = Unit::Stretch((max - *val).clamp(min, max) as f32);
                });
            })
    }
}

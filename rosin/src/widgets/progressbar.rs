use crate::{prelude::*, widgets::widget_styles};

#[derive(Copy, Clone)]
pub struct ProgressBarParams {
    min: UIParam<f64>,
    max: UIParam<f64>,
}

// TODO - use css transform so layout isn't needed
impl Default for ProgressBarParams {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressBarParams {
    pub fn new() -> Self {
        Self {
            min: UIParam::Static(0.0),
            max: UIParam::Static(1.0),
        }
    }

    pub fn min(mut self, min: impl Into<UIParam<f64>>) -> Self {
        self.min = min.into();
        self
    }

    pub fn max(mut self, max: impl Into<UIParam<f64>>) -> Self {
        self.max = max.into();
        self
    }

    pub fn view<'a, S, H>(&self, ui: &'a mut Ui<S, H>, id: NodeId, value: WeakVar<f64>) -> &'a mut Ui<S, H> {
        let min = self.min;
        let max = self.max;

        ui.node().id(id).style_sheet(widget_styles()).classes("progress-bar-bg").children(|ui| {
            ui.node().id(id!(id)).classes("progress-bar-fg").on_style(move |_, style| {
                let Some(value) = value.get() else { return };
                let min = min.get().unwrap_or(0.0);
                let max = max.get().unwrap_or(1.0);
                if max <= min {
                    return;
                }
                let normalized = ((value - min) / (max - min)).clamp(0.0, 1.0);
                style.width = Unit::Percent(normalized as f32);
            });
        })
    }
}

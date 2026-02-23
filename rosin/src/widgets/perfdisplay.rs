use std::collections::VecDeque;

use crate::{
    kurbo::{Affine, Rect},
    peniko::{Color, Fill, color},
    prelude::*,
    widgets::widget_styles,
};

// TODO - text stats, too.
pub struct PerfDisplay {
    data: Var<VecDeque<PerfInfo>>,
}

impl Default for PerfDisplay {
    fn default() -> Self {
        Self {
            data: Var::new(VecDeque::with_capacity(Self::TOTAL_FRAMES)),
        }
    }
}

impl PerfDisplay {
    const TOTAL_FRAMES: usize = 100;

    pub fn view<S, H>(&self, ui: &mut Ui<S, H>, id: NodeId) {
        let data = self.data.downgrade();

        ui.node().id(id).classes("perfdisplay").style_sheet(widget_styles()).on_canvas(move |_, ctx| {
            let Some(mut data) = data.write() else { return };

            if let Some(back) = data.back() {
                if ctx.perf_info.frame_number != back.frame_number {
                    while data.len() >= Self::TOTAL_FRAMES {
                        data.pop_front();
                    }

                    data.push_back(*ctx.perf_info);
                }
            } else {
                data.push_back(*ctx.perf_info);
            }

            let bar_width = ctx.rect.width() / Self::TOTAL_FRAMES as f64;
            let mut left_side: f64 = 0.0;
            let mut right_side: f64 = bar_width;
            let Some(max_duration) = data.iter().map(|d| d.total_time()).max() else {
                return;
            };

            for info in data.iter() {
                let mut top: f64 = ctx.rect.height();
                let mut last_top = top;

                let stages = [
                    (info.paint_time, color::palette::css::BLUE),
                    (info.scene_time, color::palette::css::ORANGE),
                    (info.layout_time, color::palette::css::YELLOW),
                    (info.style_time, color::palette::css::CYAN),
                    (info.build_time, color::palette::css::PURPLE),
                ];

                for (duration, color) in stages.iter() {
                    let segment_height = (duration.as_nanos() as f64 / max_duration.as_nanos() as f64) * ctx.rect.height();
                    top -= segment_height;

                    let shape = Rect::new(left_side.round(), top.round(), right_side.round(), last_top.round());
                    ctx.scene.fill(Fill::NonZero, Affine::IDENTITY, *color, None, &shape);

                    last_top = top;
                }

                left_side = right_side;
                right_side += bar_width;
            }

            for ms_values in [16.0, 8.0, 4.0, 2.0, 1.0].iter() {
                let line_height = (ctx.rect.height() - ((*ms_values / 1000.0) / max_duration.as_secs_f64()) * ctx.rect.height()).round();
                let rect = Rect::new(0.0, line_height, ctx.rect.width(), line_height + 1.0);
                ctx.scene.fill(Fill::NonZero, Affine::IDENTITY, Color::from_rgba8(0, 0, 0, 128), None, &rect);
            }
        });
    }
}

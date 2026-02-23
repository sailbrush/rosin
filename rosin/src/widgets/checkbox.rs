use crate::{
    kurbo::{Affine, BezPath, Rect, Stroke},
    peniko::{Color, Fill},
    prelude::*,
    widgets::widget_styles,
};

pub fn checkbox<S, H>(ui: &mut Ui<S, H>, id: NodeId, value: WeakVar<bool>, label: impl Into<UIString>) -> &mut Ui<S, H> {
    ui.node()
        .id(id)
        .classes("checkbox")
        .style_sheet(widget_styles())
        .event(On::PointerDown, move |_, ctx| {
            let Some(mut v) = value.write() else { return };
            *v = !*v;
            ctx.emit_change();
        })
        .event(On::Keyboard, move |_, ctx| {
            let Some(ev) = ctx.keyboard() else { return };

            if ev.state != KeyState::Down {
                return;
            }

            let activate = match &ev.key {
                Key::Named(NamedKey::Enter) => true,
                Key::Character(s) => s == " ",
                _ => false,
            };

            if activate {
                let Some(mut v) = value.write() else { return };
                *v = !*v;
                ctx.emit_change();
            }
        })
        .event(On::Focus, |_, _| {})
        .children(|ui| {
            ui.node()
                .id(id!(id))
                .classes("box")
                .on_style(move |_, style| {
                    if let Some(true) = value.get() {
                        if let Unit::Px(px) = &mut style.width {
                            *px += style.border_right_width.resolve(style.font_size) + style.border_left_width.resolve(style.font_size);
                        }
                        if let Unit::Px(px) = &mut style.height {
                            *px += style.border_top_width.resolve(style.font_size) + style.border_bottom_width.resolve(style.font_size);
                        }

                        style.border_top_width = Length::ZERO;
                        style.border_right_width = Length::ZERO;
                        style.border_bottom_width = Length::ZERO;
                        style.border_left_width = Length::ZERO;
                    }
                })
                .on_canvas(move |_, ctx| {
                    if let Some(true) = value.get() {
                        let shape = Rect::from_origin_size((0.0, 0.0), ctx.rect.rect().size());
                        ctx.scene.fill(Fill::NonZero, Affine::IDENTITY, ctx.style.color, None, &shape);

                        let p0 = (15.0 * ctx.rect.width() / 48.0, ctx.rect.height() / 2.0);
                        let p1 = (6.0 * ctx.rect.width() / 12.0, 7.0 * ctx.rect.height() / 10.0);
                        let p2 = (18.0 * ctx.rect.width() / 24.0, ctx.rect.height() / 3.0);

                        let mut check = BezPath::new();
                        check.move_to(p0);
                        check.line_to(p1);
                        check.line_to(p2);

                        ctx.scene.stroke(&Stroke::new(3.0), Affine::IDENTITY, Color::WHITE, None, &check);
                    }
                });
            ui.node().id(id!(id)).text(label).classes("checkbox_label");
        })
}

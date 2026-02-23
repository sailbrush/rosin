use rosin_core::{css::Length, vello::kurbo::Vec2};

use crate::{prelude::*, widgets::widget_styles};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Transform {
    pos: Vec2,
    width: f32,
    height: f32,
}

// TODO - use the builder pattern

pub struct DropDown<T: Send + Sync + PartialEq + Copy + 'static> {
    choices: Var<Vec<(T, LocalizedString)>>,
    transform: Var<Option<Transform>>,
    below: Var<bool>,
    arrow: Var<&'static str>,
}

impl<T: Send + Sync + PartialEq + Copy + 'static> DropDown<T> {
    pub fn new<const N: usize>(list: [(T, &LocalizedString); N]) -> Self {
        let mut choices = Vec::with_capacity(N);

        for (choice, label) in list {
            choices.push((choice, label.clone()));
        }

        Self {
            choices: Var::new(choices),
            transform: Var::new(None),
            below: Var::new(true),
            arrow: Var::new("▼"),
        }
    }

    pub fn view<'a, S, H>(&self, ui: &'a mut Ui<S, H>, id: NodeId, value: WeakVar<T>) -> &'a mut Ui<S, H> {
        let transform = self.transform.downgrade();
        let below = self.below.downgrade();
        let arrow = self.arrow.downgrade();
        let choices = self.choices.downgrade();

        ui.node()
            .id(id)
            .classes("dropdown")
            .style_sheet(widget_styles())
            .event(On::PointerDown, {
                let choices = *self.choices;
                move |_, ctx| {
                    let menu_height = choices.get().unwrap().len() as f64 * ctx.rect().height();
                    let menu_will_fit_below = ctx.rect().origin().y + ctx.rect().height() + menu_height <= ctx.viewport_size().height;
                    below.set(menu_will_fit_below);
                    let offset_y = if menu_will_fit_below {
                        arrow.set("▼");
                        ctx.rect().height() - ctx.style().border_bottom_width.resolve(ctx.style().font_size) as f64
                    } else {
                        arrow.set("▲");
                        -menu_height
                    };

                    transform.set(Some(Transform {
                        pos: ctx.rect().origin().to_vec2() + Vec2::new(0.0, offset_y),
                        width: ctx.rect().width() as f32,
                        height: choices.get().unwrap().len() as f32 * ctx.rect().height() as f32,
                    }));

                    ctx.stop_propagation();
                    ctx.set_focus(ctx.id());
                    ctx.set_active(ctx.id());
                }
            })
            .event(On::PointerWheel, {
                move |_, ctx| {
                    if transform.get().flatten().is_some() {
                        // Intercept scrolling events when a dropdown is active
                        ctx.stop_propagation();
                    }
                }
            })
            .event(On::PointerLeave, move |_, ctx| {
                if ctx.is_focused() {
                    transform.set(None);
                    ctx.set_focus(None);
                }
                ctx.set_active(None);
            })
            .on_style(move |_, style| {
                if transform.get().flatten().is_none() {
                    return;
                }
                if let Some(true) = below.get() {
                    style.border_bottom_left_radius = Length::ZERO;
                    style.border_bottom_right_radius = Length::ZERO;
                } else {
                    style.border_top_left_radius = Length::ZERO;
                    style.border_top_right_radius = Length::ZERO;
                }
            })
            .children(move |ui| {
                ui.node().classes("text").text(UIString::deferred(move |translation_map| {
                    choices
                        .get()
                        .unwrap()
                        .iter()
                        .find(|(c, _)| *c == value.get().unwrap())
                        .map(|(_, string)| string.resolve(translation_map).to_string())
                        .unwrap_or_default()
                }));
                ui.node().text(arrow).classes("arrow");
                ui.node()
                    .id(id!(id))
                    .classes("menu")
                    .on_style(move |_, style| {
                        if let Some(ref transform) = transform.get().flatten() {
                            style.display = Some(Direction::Column);
                            style.position = Position::Fixed;
                            style.top = Unit::Px(transform.pos.y as f32);
                            style.left = Unit::Px(transform.pos.x as f32);
                            style.width = Unit::Px(
                                transform.width - style.border_left_width.resolve(style.font_size) - style.border_right_width.resolve(style.font_size),
                            );
                            style.height = Unit::Px(transform.height);

                            if let Some(true) = below.get() {
                                style.border_top_left_radius = Length::ZERO;
                                style.border_top_right_radius = Length::ZERO;
                            } else {
                                style.border_bottom_left_radius = Length::ZERO;
                                style.border_bottom_right_radius = Length::ZERO;
                            }
                        }
                    })
                    .children(move |ui| {
                        for (count, (choice, label)) in self.choices.get().iter().enumerate() {
                            let choice = *choice;
                            ui.node()
                                .id(id!(id, count as u64))
                                .text(label.clone())
                                .classes("item")
                                .event(On::PointerUp, move |_, ctx| {
                                    ctx.emit_change();
                                    transform.set(None);
                                    value.set(choice);
                                });
                        }
                    });
            })
    }
}

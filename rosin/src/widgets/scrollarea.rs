use crate::{kurbo::Vec2, prelude::*, widgets::widget_styles};

// TODO - scroll bars and stuff
#[cfg_attr(feature = "hot-reload", derive(serde::Deserialize, serde::Serialize, TypeHash), serde(default))]
#[derive(Default, Debug)]
pub struct ScrollArea {
    offset: Var<Vec2>,
}

impl ScrollArea {
    pub fn reset(&self) {
        self.offset.set(Vec2::ZERO);
    }

    pub fn view<'a, S, H>(&self, ui: &'a mut Ui<S, H>, id: NodeId, func: impl FnOnce(&mut Ui<S, H>)) -> &'a mut Ui<S, H> {
        let offset = self.offset.downgrade();
        ui.node()
            .id(id)
            .classes("scroll-area")
            .offset(offset)
            .style_sheet(widget_styles())
            .event(On::PointerWheel, move |_, ctx| {
                let Some(pointer) = ctx.pointer() else { return };
                let Some(mut offset) = offset.write() else { return };

                *offset -= pointer.wheel_delta;
            })
            .children(|ui| func(ui))
    }
}

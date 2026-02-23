use rosin_core::data::UIString;

use crate::{prelude::*, widgets::widget_styles};

pub fn label<S, H>(ui: &mut Ui<S, H>, id: NodeId, text: impl Into<UIString>) -> &mut Ui<S, H> {
    ui.node().id(id).classes("label").style_sheet(widget_styles()).text(text)
}

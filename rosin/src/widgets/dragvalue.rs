use crate::{prelude::*, widgets::widget_styles};

// TODO - configurable display rounding
//      - double click to type specific value
//      - min/max
pub fn dragvalue<S>(ui: &mut Ui<S, WindowHandle>, id: NodeId, value: WeakVar<f64>) -> &mut Ui<S, WindowHandle> {
    ui.node()
        .id(id)
        .classes("drag-value")
        .style_sheet(widget_styles())
        .text(ui_format!(value, "{:.2}"))
        .event(On::PointerDown, |_, ctx| {
            let Some(ev) = ctx.pointer() else { return };
            if ev.button.is_primary() && !ev.did_focus_window {
                ctx.begin_pointer_capture();
                ctx.platform().set_cursor(CursorType::EWResize);
            }
        })
        .event(On::PointerUp, |_, ctx| {
            ctx.end_pointer_capture();
            ctx.platform().set_cursor(CursorType::Default);
        })
        .event(On::PointerMove, move |_, ctx| {
            if ctx.is_pointer_captured()
                && let Some(delta) = ctx.pointer_delta()
                && let Some(mut value) = value.write()
            {
                *value += delta.x - delta.y;
            }
        })
}

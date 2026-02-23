use std::rc::Rc;

use crate::{
    accesskit::{Action as AxAction, Role},
    kurbo::Shape,
    prelude::*,
    widgets::widget_styles,
};

pub fn button<S, H>(ui: &mut Ui<S, H>, id: NodeId, text: impl Into<UIString>, callback: impl Fn(&mut S, &mut EventCtx<H>) + 'static) -> &mut Ui<S, H> {
    let callback = Rc::new(callback);
    let cb_pointer = callback.clone();
    let cb_keyboard = callback.clone();
    let cb_accessibility = callback.clone();

    ui.node()
        .id(id)
        .text(text)
        .classes("button")
        .style_sheet(widget_styles())
        .on_accessibility(|_s, ctx| {
            ctx.node.set_role(Role::Button);
            ctx.node.add_action(AxAction::Click);

            if let Some(text) = ctx.text
                && let Some(resolved) = text.resolve(&ctx.translation_map)
            {
                ctx.node.set_label(resolved);
            }
        })
        .event(On::AccessibilityAction, move |s, ctx| {
            let Some(req) = ctx.action_request() else { return };

            if req.action == AxAction::Click {
                ctx.set_focus(ctx.id());
                ctx.emit_change();
                (cb_accessibility)(s, ctx);
            }
        })
        .event(On::PointerDown, |_, ctx| {
            ctx.set_focus(None);
            ctx.begin_pointer_capture();
            ctx.set_active(ctx.id());
        })
        .event(On::PointerLeave, |_, ctx| {
            if ctx.is_pointer_captured() {
                ctx.set_active(None);
            }
        })
        .event(On::PointerEnter, |_, ctx| {
            if ctx.is_pointer_captured() {
                ctx.set_active(ctx.id());
            }
        })
        .event(On::PointerUp, move |s, ctx| {
            if ctx.is_pointer_captured() {
                ctx.end_pointer_capture();
                ctx.set_active(None);

                let Some(pointer) = ctx.pointer() else {
                    return;
                };

                if ctx.rect().contains(pointer.viewport_pos) {
                    ctx.emit_change();
                    (cb_pointer)(s, ctx);
                }
            }
        })
        .event(On::Keyboard, move |s, ctx| {
            let Some(ev) = ctx.keyboard() else { return };

            let activate = match &ev.key {
                Key::Named(NamedKey::Enter) => true,
                Key::Character(s) => s == " ",
                _ => false,
            };

            if ev.state == KeyState::Down && activate {
                ctx.emit_change();
                (cb_keyboard)(s, ctx);
            }
        })
        .event(On::Focus, |_, _| {})
}

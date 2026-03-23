use std::rc::Rc;

use crate::{
    accesskit::{Action as AxAction, Role},
    prelude::*,
    widgets::widget_styles,
};

#[cfg_attr(feature = "hot-reload", derive(serde::Deserialize, serde::Serialize, TypeHash), serde(default))]
#[derive(Default, Debug)]
pub struct Tabs {
    active_nid: Var<Option<NodeId>>,
    first_nid: Var<Option<NodeId>>,
}

impl Tabs {
    pub fn set_first(&self, id: NodeId) {
        self.first_nid.set(Some(id));
    }

    pub fn reset<H>(&self, ctx: &mut EventCtx<H>) {
        if let Some(prev_id) = self.active_nid.get() {
            ctx.unset_active_node(prev_id);
        }
        if let Some(first_id) = self.first_nid.get() {
            ctx.set_active_node(first_id);
            self.active_nid.set(Some(first_id));
        }
    }

    pub fn tab<S, H>(&self, ui: &mut Ui<S, H>, id: NodeId, label: impl Into<UIString>, on_select: impl Fn(&mut S, &mut EventCtx<H>) + 'static) {
        let on_select = Rc::new(on_select);
        let on_select_ak = on_select.clone();
        let active_nid = self.active_nid.downgrade();

        ui.node()
            .id(id)
            .classes("tab")
            .style_sheet(widget_styles())
            .text(label)
            .on_accessibility(|_s, ctx| {
                ctx.node.set_role(Role::Tab);
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
                    if let Some(prev_id) = active_nid.get().flatten() {
                        ctx.unset_active_node(prev_id);
                    }
                    ctx.set_active();
                    active_nid.set(ctx.id());
                    (on_select_ak)(s, ctx);
                }
            })
            .event(On::PointerDown, move |s, ctx| {
                if let Some(prev_id) = active_nid.get().flatten() {
                    ctx.unset_active_node(prev_id);
                }
                ctx.set_active();
                active_nid.set(ctx.id());
                (on_select)(s, ctx);
            })
            .event(On::Focus, |_, _| {});
    }
}

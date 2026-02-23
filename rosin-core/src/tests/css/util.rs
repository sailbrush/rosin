pub use std::{str::FromStr, sync::Arc};

pub use kurbo::{Affine, Size, Vec2};
pub use unic_langid::langid;
pub use vello::peniko::{
    Color,
    color::{self, Srgb},
};

pub use crate::prelude::*;

pub(crate) fn single_node_tree(state: &Stylesheet, ui: &mut Ui<Stylesheet, ()>) {
    ui.node().style_sheet(state).classes("root");
}

pub(crate) fn one_child_tree(state: &Stylesheet, ui: &mut Ui<Stylesheet, ()>) {
    ui.node().style_sheet(state).classes("parent").children(|ui| {
        ui.node().classes("child");
    });
}

pub(crate) fn two_child_tree(state: &Stylesheet, ui: &mut Ui<Stylesheet, ()>) {
    ui.node().style_sheet(state).classes("parent").children(|ui| {
        ui.node().classes("left child");
        ui.node().classes("right child");
    });
}

pub(crate) fn apply_css_to_tree(css: &str, view: fn(&Stylesheet, &mut Ui<Stylesheet, ()>)) -> Vec<Style> {
    let size = Size::new(500.0, 500.0);
    let scale = Vec2::new(1.0, 1.0);
    let translation_map = TranslationMap::new(langid!("en-US"));
    let mut viewport = Viewport::new(view, size, scale, translation_map);
    let state = Stylesheet::from_str(css).unwrap();
    viewport.frame(&state);
    viewport.curr_tree.style_cache.clone()
}

use druid_shell::kurbo::Rect;
use druid_shell::piet::{Color, Piet, RenderContext};

use crate::layout::Layout;
use crate::style::Style;
use crate::tree::ArrayNode;

pub(crate) fn render<T>(tree: &[ArrayNode<T>], layouts: &[Layout], piet: &mut Piet) {
    for (i, node) in tree.iter().rev().enumerate() {
        let rect = Rect::new(
            layouts[i].position.x as f64,
            layouts[i].position.y as f64,
            layouts[i].position.x as f64 + layouts[i].size.width as f64,
            layouts[i].position.y as f64 + layouts[i].size.height as f64,
        );

        piet.fill(rect, &tree[i].style.background_color);
    }
}

use crate::layout::Layout;
use crate::tree::ArrayNode;

use raqote::*;

pub(crate) fn render<T>(tree: &[ArrayNode<T>], layout: &[Layout]) -> DrawTarget {
    let mut dt = DrawTarget::new(layout[0].size.width as i32, layout[0].size.height as i32);
    for (i, tree_node) in tree.iter().rev().enumerate() {
        let color = tree_node.style.background_color;

        dt.fill_rect(
            layout[i].position.x,
            layout[i].position.y,
            layout[i].size.width,
            layout[i].size.height,
            &Source::Solid(SolidSource::from_unpremultiplied_argb(
                color.alpha,
                color.red,
                color.green,
                color.blue,
            )),
            &DrawOptions::default(),
        );
    }
    dt
}

#![forbid(unsafe_code)]

use crate::layout::Layout;
use crate::tree::ArrayNode;

use raqote::*;

pub(crate) fn render<T>(tree: &[ArrayNode<T>], layout: &[Layout]) -> DrawTarget {
    let mut dt = DrawTarget::new(layout[0].size.width as i32, layout[0].size.height as i32);
    render_node(0, 0.0, 0.0, &mut dt, tree, layout);
    dt
}

fn render_node<T>(id: usize, offset_x: f32, offset_y: f32, dt: &mut DrawTarget, tree: &[ArrayNode<T>], layout: &[Layout]) {
    let color = tree[id].style.background_color;
    dt.fill_rect(
        layout[id].position.x + offset_x,
        layout[id].position.y + offset_y,
        layout[id].size.width,
        layout[id].size.height,
        &Source::Solid(SolidSource::from_unpremultiplied_argb(
            color.alpha,
            color.red,
            color.green,
            color.blue,
        )),
        &DrawOptions::default(),
    );

    for i in tree[id].child_ids() {
        render_node(
            i,
            layout[id].position.x + offset_x,
            layout[id].position.y + offset_y,
            dt,
            tree,
            layout,
        );
    }
}

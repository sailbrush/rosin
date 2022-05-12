#![forbid(unsafe_code)]

use crate::layout::Layout;
use crate::prelude::*;
use crate::tree::ArrayNode;

use bumpalo::{
    collections::{CollectIn, Vec as BumpVec},
    Bump,
};
use druid_shell::{
    kurbo::{Affine, RoundedRect},
    piet::{Piet, RenderContext},
};

pub(crate) fn draw<S, H>(temp: &Bump, state: &S, tree: &[ArrayNode<S, H>], styles: &[Style], layout: &[Layout], piet: &mut Piet<'_>) {
    draw_inner(temp, state, tree, styles, layout, piet, 0..1);
}

fn draw_inner<S, H>(
    temp: &Bump,
    state: &S,
    tree: &[ArrayNode<S, H>],
    styles: &[Style],
    layout: &[Layout],
    piet: &mut Piet<'_>,
    range: std::ops::Range<usize>,
) {
    let mut ids: BumpVec<usize> = range.collect_in(temp);
    ids.sort_by(|a, b| styles[*a].z_index.partial_cmp(&styles[*b].z_index).unwrap());

    for id in ids {
        let style = &styles[id];

        // ---------- Draw the box ----------
        let bg_color = &style.background_color;

        // TODO - use all border colors
        let border_color = &style.border_top_color;

        let path = RoundedRect::new(
            layout[id].position.x as f64,
            layout[id].position.y as f64,
            layout[id].position.x as f64 + layout[id].size.width as f64,
            layout[id].position.y as f64 + layout[id].size.height as f64,
            style.border_top_left_radius.into(),
        );

        if let Some(shadows) = &style.box_shadow {
            for shadow in shadows.iter() {
                let blur = shadow.blur.resolve(style.font_size);
                if blur < 1.0 {
                    piet.fill(path.rect(), bg_color);
                } else {
                    piet.blurred_rect(path.rect(), blur, shadow.color.as_ref().unwrap_or(&style.color));
                }
            }
        }
        piet.fill(path, bg_color);
        if let Some(gradients) = &style.background_image {
            for gradient in gradients.iter() {
                piet.fill(path, &gradient.resolve(layout[id].size.width, layout[id].size.height));
            }
        }

        // Call on_draw()
        if let Some(on_draw) = &tree[id].draw_callback {
            piet.save().unwrap();
            piet.clip(path);

            piet.transform(Affine::translate((
                layout[id].position.x as f64 + style.border_left_width as f64,
                layout[id].position.y as f64 + style.border_top_width as f64,
            )));

            let mut ctx = DrawCtx {
                piet,
                style,
                width: layout[id].size.width as f64,
                height: layout[id].size.height as f64,
                must_draw: true, // TODO - caching system
            };

            (*on_draw)(state, &mut ctx);

            piet.restore().unwrap();
        }

        piet.stroke(path, border_color, style.border_top_width.into());

        if let Some(child_ids) = tree[id].child_ids() {
            draw_inner(temp, state, tree, styles, layout, piet, child_ids);
        }
    }
}

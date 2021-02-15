use crate::layout::Layout;
use crate::style::Style;
use crate::tree::ArrayNode;

use raqote::*;
use winit::dpi::PhysicalSize;

/*
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
*/

pub(crate) fn render(size: PhysicalSize<u32>) -> DrawTarget {
    let w = size.width as f32;
    let h = size.height as f32;

    let mut dt = DrawTarget::new(size.width as i32, size.height as i32);
    let mut pb = PathBuilder::new();
    pb.move_to(w / 2., 0.);
    pb.quad_to(0., 0., 80., 200.);
    pb.quad_to(150., 180., w, h);
    pb.close();
    let path = pb.finish();

    let gradient = Source::new_radial_gradient(
        Gradient {
            stops: vec![
                GradientStop {
                    position: 0.2,
                    color: Color::new(0xff, 0, 0xff, 0),
                },
                GradientStop {
                    position: 0.8,
                    color: Color::new(0xff, 0xff, 0xff, 0xff),
                },
                GradientStop {
                    position: 1.,
                    color: Color::new(0xff, 0xff, 0, 0xff),
                },
            ],
        },
        Point::new(150., 150.),
        128.,
        Spread::Pad,
    );
    dt.fill(&path, &gradient, &DrawOptions::new());
    dt
}

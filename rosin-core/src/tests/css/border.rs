use super::util::*;

#[test]
fn border_bottom() {
    use crate::css::Length;

    // Normal use
    let styles = apply_css_to_tree(".root { border-bottom: 2px #F00; }", single_node_tree);
    assert_eq!(styles[0].border_bottom_width, Length::Px(2.0));
    assert_eq!(styles[0].border_bottom_color, color::palette::css::RED);

    // Initial
    let styles = apply_css_to_tree(".parent { border-bottom: 3px rgb(0, 0, 255); } .child { border-bottom: initial; }", one_child_tree);
    assert_eq!(styles[0].border_bottom_width, Length::Px(3.0));
    assert_eq!(styles[0].border_bottom_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_bottom_width, Style::default().border_bottom_width);
    assert_eq!(styles[1].border_bottom_color, Style::default().border_bottom_color);

    // Inherit
    let styles = apply_css_to_tree(".parent { border-bottom: 3px blue; } .child { border-bottom: inherit; }", one_child_tree);
    assert_eq!(styles[0].border_bottom_width, Length::Px(3.0));
    assert_eq!(styles[0].border_bottom_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_bottom_width, Length::Px(3.0));
    assert_eq!(styles[1].border_bottom_color, color::palette::css::BLUE);

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { border-bottom: 4px green; }", one_child_tree);
    assert_eq!(styles[0].border_bottom_width, Length::Px(4.0));
    assert_eq!(styles[0].border_bottom_color, color::palette::css::GREEN);
    assert_eq!(styles[1].border_bottom_width, Style::default().border_bottom_width);
    assert_eq!(styles[1].border_bottom_color, Style::default().border_bottom_color);
}

#[test]
fn border_color() {
    // (no changes needed; colors stayed the same)

    // Normal use
    let styles = apply_css_to_tree(".root { border-color: #F00; }", single_node_tree);
    assert_eq!(styles[0].border_bottom_color, color::palette::css::RED);
    assert_eq!(styles[0].border_left_color, color::palette::css::RED);
    assert_eq!(styles[0].border_right_color, color::palette::css::RED);
    assert_eq!(styles[0].border_top_color, color::palette::css::RED);

    // Initial
    let styles = apply_css_to_tree(".parent { border-color: rgb(0, 0, 255); } .child { border-color: initial; }", one_child_tree);
    assert_eq!(styles[0].border_bottom_color, color::palette::css::BLUE);
    assert_eq!(styles[0].border_left_color, color::palette::css::BLUE);
    assert_eq!(styles[0].border_right_color, color::palette::css::BLUE);
    assert_eq!(styles[0].border_top_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_bottom_color, Style::default().border_bottom_color);
    assert_eq!(styles[1].border_left_color, Style::default().border_left_color);
    assert_eq!(styles[1].border_right_color, Style::default().border_right_color);
    assert_eq!(styles[1].border_top_color, Style::default().border_top_color);

    // Inherit
    let styles = apply_css_to_tree(".parent { border-color: blue; } .child { border-color: inherit; }", one_child_tree);
    assert_eq!(styles[0].border_bottom_color, color::palette::css::BLUE);
    assert_eq!(styles[0].border_left_color, color::palette::css::BLUE);
    assert_eq!(styles[0].border_right_color, color::palette::css::BLUE);
    assert_eq!(styles[0].border_top_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_bottom_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_left_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_right_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_top_color, color::palette::css::BLUE);

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { border-color: green; }", one_child_tree);
    assert_eq!(styles[0].border_bottom_color, color::palette::css::GREEN);
    assert_eq!(styles[0].border_left_color, color::palette::css::GREEN);
    assert_eq!(styles[0].border_right_color, color::palette::css::GREEN);
    assert_eq!(styles[0].border_top_color, color::palette::css::GREEN);
    assert_eq!(styles[1].border_bottom_color, Style::default().border_bottom_color);
    assert_eq!(styles[1].border_left_color, Style::default().border_left_color);
    assert_eq!(styles[1].border_right_color, Style::default().border_right_color);
    assert_eq!(styles[1].border_top_color, Style::default().border_top_color);
}

#[test]
fn border_left() {
    use crate::css::Length;

    // Normal use
    let styles = apply_css_to_tree(".root { border-left: 2px #F00; }", single_node_tree);
    assert_eq!(styles[0].border_left_width, Length::Px(2.0));
    assert_eq!(styles[0].border_left_color, color::palette::css::RED);

    // Initial
    let styles = apply_css_to_tree(".parent { border-left: 3px rgb(0, 0, 255); } .child { border-left: initial; }", one_child_tree);
    assert_eq!(styles[0].border_left_width, Length::Px(3.0));
    assert_eq!(styles[0].border_left_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_left_width, Style::default().border_left_width);
    assert_eq!(styles[1].border_left_color, Style::default().border_left_color);

    // Inherit
    let styles = apply_css_to_tree(".parent { border-left: 3px blue; } .child { border-left: inherit; }", one_child_tree);
    assert_eq!(styles[0].border_left_width, Length::Px(3.0));
    assert_eq!(styles[0].border_left_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_left_width, Length::Px(3.0));
    assert_eq!(styles[1].border_left_color, color::palette::css::BLUE);

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { border-left: 4px green; }", one_child_tree);
    assert_eq!(styles[0].border_left_width, Length::Px(4.0));
    assert_eq!(styles[0].border_left_color, color::palette::css::GREEN);
    assert_eq!(styles[1].border_left_width, Style::default().border_left_width);
    assert_eq!(styles[1].border_left_color, Style::default().border_left_color);
}

#[test]
fn border_radius() {
    use crate::css::Length;

    // Normal use
    let styles = apply_css_to_tree(".root { border-radius: 10px; }", single_node_tree);
    assert_eq!(styles[0].border_top_left_radius, Length::Px(10.0));
    assert_eq!(styles[0].border_top_right_radius, Length::Px(10.0));
    assert_eq!(styles[0].border_bottom_right_radius, Length::Px(10.0));
    assert_eq!(styles[0].border_bottom_left_radius, Length::Px(10.0));

    // Initial
    let styles = apply_css_to_tree(".parent { border-radius: 15px; } .child { border-radius: initial; }", one_child_tree);
    assert_eq!(styles[0].border_top_left_radius, Length::Px(15.0));
    assert_eq!(styles[0].border_top_right_radius, Length::Px(15.0));
    assert_eq!(styles[0].border_bottom_right_radius, Length::Px(15.0));
    assert_eq!(styles[0].border_bottom_left_radius, Length::Px(15.0));
    assert_eq!(styles[1].border_top_left_radius, Style::default().border_top_left_radius);
    assert_eq!(styles[1].border_top_right_radius, Style::default().border_top_right_radius);
    assert_eq!(styles[1].border_bottom_right_radius, Style::default().border_bottom_right_radius);
    assert_eq!(styles[1].border_bottom_left_radius, Style::default().border_bottom_left_radius);

    // Inherit
    let styles = apply_css_to_tree(".parent { border-radius: 20px; } .child { border-radius: inherit; }", one_child_tree);
    assert_eq!(styles[0].border_top_left_radius, Length::Px(20.0));
    assert_eq!(styles[0].border_top_right_radius, Length::Px(20.0));
    assert_eq!(styles[0].border_bottom_right_radius, Length::Px(20.0));
    assert_eq!(styles[0].border_bottom_left_radius, Length::Px(20.0));
    assert_eq!(styles[1].border_top_left_radius, Length::Px(20.0));
    assert_eq!(styles[1].border_top_right_radius, Length::Px(20.0));
    assert_eq!(styles[1].border_bottom_right_radius, Length::Px(20.0));
    assert_eq!(styles[1].border_bottom_left_radius, Length::Px(20.0));

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { border-radius: 25px; }", one_child_tree);
    assert_eq!(styles[0].border_top_left_radius, Length::Px(25.0));
    assert_eq!(styles[0].border_top_right_radius, Length::Px(25.0));
    assert_eq!(styles[0].border_bottom_right_radius, Length::Px(25.0));
    assert_eq!(styles[0].border_bottom_left_radius, Length::Px(25.0));
    assert_eq!(styles[1].border_top_left_radius, Style::default().border_top_left_radius);
    assert_eq!(styles[1].border_top_right_radius, Style::default().border_top_right_radius);
    assert_eq!(styles[1].border_bottom_right_radius, Style::default().border_bottom_right_radius);
    assert_eq!(styles[1].border_bottom_left_radius, Style::default().border_bottom_left_radius);
}

#[test]
fn border_right() {
    use crate::css::Length;

    // Normal use
    let styles = apply_css_to_tree(".root { border-right: 2px #F00; }", single_node_tree);
    assert_eq!(styles[0].border_right_width, Length::Px(2.0));
    assert_eq!(styles[0].border_right_color, color::palette::css::RED);

    // Initial
    let styles = apply_css_to_tree(".parent { border-right: 3px rgb(0, 0, 255); } .child { border-right: initial; }", one_child_tree);
    assert_eq!(styles[0].border_right_width, Length::Px(3.0));
    assert_eq!(styles[0].border_right_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_right_width, Style::default().border_right_width);
    assert_eq!(styles[1].border_right_color, Style::default().border_right_color);

    // Inherit
    let styles = apply_css_to_tree(".parent { border-right: 3px blue; } .child { border-right: inherit; }", one_child_tree);
    assert_eq!(styles[0].border_right_width, Length::Px(3.0));
    assert_eq!(styles[0].border_right_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_right_width, Length::Px(3.0));
    assert_eq!(styles[1].border_right_color, color::palette::css::BLUE);

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { border-right: 4px green; }", one_child_tree);
    assert_eq!(styles[0].border_right_width, Length::Px(4.0));
    assert_eq!(styles[0].border_right_color, color::palette::css::GREEN);
    assert_eq!(styles[1].border_right_width, Style::default().border_right_width);
    assert_eq!(styles[1].border_right_color, Style::default().border_right_color);
}

#[test]
fn border_top() {
    use crate::css::Length;

    // Normal use
    let styles = apply_css_to_tree(".root { border-top: 2px #F00; }", single_node_tree);
    assert_eq!(styles[0].border_top_width, Length::Px(2.0));
    assert_eq!(styles[0].border_top_color, color::palette::css::RED);

    // Initial
    let styles = apply_css_to_tree(".parent { border-top: 3px rgb(0, 0, 255); } .child { border-top: initial; }", one_child_tree);
    assert_eq!(styles[0].border_top_width, Length::Px(3.0));
    assert_eq!(styles[0].border_top_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_top_width, Style::default().border_top_width);
    assert_eq!(styles[1].border_top_color, Style::default().border_top_color);

    // Inherit
    let styles = apply_css_to_tree(".parent { border-top: 3px blue; } .child { border-top: inherit; }", one_child_tree);
    assert_eq!(styles[0].border_top_width, Length::Px(3.0));
    assert_eq!(styles[0].border_top_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_top_width, Length::Px(3.0));
    assert_eq!(styles[1].border_top_color, color::palette::css::BLUE);

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { border-top: 4px green; }", one_child_tree);
    assert_eq!(styles[0].border_top_width, Length::Px(4.0));
    assert_eq!(styles[0].border_top_color, color::palette::css::GREEN);
    assert_eq!(styles[1].border_top_width, Style::default().border_top_width);
    assert_eq!(styles[1].border_top_color, Style::default().border_top_color);
}

#[test]
fn border_width() {
    use crate::css::Length;

    // Normal use
    let styles = apply_css_to_tree(".root { border-width: 2px; }", single_node_tree);
    assert_eq!(styles[0].border_top_width, Length::Px(2.0));
    assert_eq!(styles[0].border_right_width, Length::Px(2.0));
    assert_eq!(styles[0].border_bottom_width, Length::Px(2.0));
    assert_eq!(styles[0].border_left_width, Length::Px(2.0));

    // Initial
    let styles = apply_css_to_tree(".parent { border-width: 3px; } .child { border-width: initial; }", one_child_tree);
    assert_eq!(styles[0].border_top_width, Length::Px(3.0));
    assert_eq!(styles[0].border_right_width, Length::Px(3.0));
    assert_eq!(styles[0].border_bottom_width, Length::Px(3.0));
    assert_eq!(styles[0].border_left_width, Length::Px(3.0));
    assert_eq!(styles[1].border_top_width, Style::default().border_top_width);
    assert_eq!(styles[1].border_right_width, Style::default().border_right_width);
    assert_eq!(styles[1].border_bottom_width, Style::default().border_bottom_width);
    assert_eq!(styles[1].border_left_width, Style::default().border_left_width);

    // Inherit
    let styles = apply_css_to_tree(".parent { border-width: 4px; } .child { border-width: inherit; }", one_child_tree);
    assert_eq!(styles[0].border_top_width, Length::Px(4.0));
    assert_eq!(styles[0].border_right_width, Length::Px(4.0));
    assert_eq!(styles[0].border_bottom_width, Length::Px(4.0));
    assert_eq!(styles[0].border_left_width, Length::Px(4.0));
    assert_eq!(styles[1].border_top_width, Length::Px(4.0));
    assert_eq!(styles[1].border_right_width, Length::Px(4.0));
    assert_eq!(styles[1].border_bottom_width, Length::Px(4.0));
    assert_eq!(styles[1].border_left_width, Length::Px(4.0));

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { border-width: 5px; }", one_child_tree);
    assert_eq!(styles[0].border_top_width, Length::Px(5.0));
    assert_eq!(styles[0].border_right_width, Length::Px(5.0));
    assert_eq!(styles[0].border_bottom_width, Length::Px(5.0));
    assert_eq!(styles[0].border_left_width, Length::Px(5.0));
    assert_eq!(styles[1].border_top_width, Style::default().border_top_width);
    assert_eq!(styles[1].border_right_width, Style::default().border_right_width);
    assert_eq!(styles[1].border_bottom_width, Style::default().border_bottom_width);
    assert_eq!(styles[1].border_left_width, Style::default().border_left_width);
}

#[test]
fn border() {
    use crate::css::Length;

    // Normal use
    let styles = apply_css_to_tree(".root { border: 2px #F00; }", single_node_tree);
    assert_eq!(styles[0].border_bottom_width, Length::Px(2.0));
    assert_eq!(styles[0].border_left_width, Length::Px(2.0));
    assert_eq!(styles[0].border_right_width, Length::Px(2.0));
    assert_eq!(styles[0].border_top_width, Length::Px(2.0));
    assert_eq!(styles[0].border_bottom_color, color::palette::css::RED);
    assert_eq!(styles[0].border_left_color, color::palette::css::RED);
    assert_eq!(styles[0].border_right_color, color::palette::css::RED);
    assert_eq!(styles[0].border_top_color, color::palette::css::RED);

    // Initial
    let styles = apply_css_to_tree(".parent { border: 3px rgb(0, 0, 255); } .child { border: initial; }", one_child_tree);
    assert_eq!(styles[0].border_bottom_width, Length::Px(3.0));
    assert_eq!(styles[0].border_left_width, Length::Px(3.0));
    assert_eq!(styles[0].border_right_width, Length::Px(3.0));
    assert_eq!(styles[0].border_top_width, Length::Px(3.0));
    assert_eq!(styles[0].border_bottom_color, color::palette::css::BLUE);
    assert_eq!(styles[0].border_left_color, color::palette::css::BLUE);
    assert_eq!(styles[0].border_right_color, color::palette::css::BLUE);
    assert_eq!(styles[0].border_top_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_bottom_color, Style::default().border_bottom_color);
    assert_eq!(styles[1].border_left_color, Style::default().border_left_color);
    assert_eq!(styles[1].border_right_color, Style::default().border_right_color);
    assert_eq!(styles[1].border_top_color, Style::default().border_top_color);
    assert_eq!(styles[1].border_bottom_width, Style::default().border_bottom_width);
    assert_eq!(styles[1].border_left_width, Style::default().border_left_width);
    assert_eq!(styles[1].border_right_width, Style::default().border_right_width);
    assert_eq!(styles[1].border_top_width, Style::default().border_top_width);

    // Inherit
    let styles = apply_css_to_tree(".parent { border: 3px blue; } .child { border: inherit; }", one_child_tree);
    assert_eq!(styles[0].border_bottom_width, Length::Px(3.0));
    assert_eq!(styles[0].border_left_width, Length::Px(3.0));
    assert_eq!(styles[0].border_right_width, Length::Px(3.0));
    assert_eq!(styles[0].border_top_width, Length::Px(3.0));
    assert_eq!(styles[0].border_bottom_color, color::palette::css::BLUE);
    assert_eq!(styles[0].border_left_color, color::palette::css::BLUE);
    assert_eq!(styles[0].border_right_color, color::palette::css::BLUE);
    assert_eq!(styles[0].border_top_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_bottom_width, Length::Px(3.0));
    assert_eq!(styles[1].border_left_width, Length::Px(3.0));
    assert_eq!(styles[1].border_right_width, Length::Px(3.0));
    assert_eq!(styles[1].border_top_width, Length::Px(3.0));
    assert_eq!(styles[1].border_bottom_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_left_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_right_color, color::palette::css::BLUE);
    assert_eq!(styles[1].border_top_color, color::palette::css::BLUE);

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { border: 4px green; }", one_child_tree);
    assert_eq!(styles[0].border_bottom_width, Length::Px(4.0));
    assert_eq!(styles[0].border_left_width, Length::Px(4.0));
    assert_eq!(styles[0].border_right_width, Length::Px(4.0));
    assert_eq!(styles[0].border_top_width, Length::Px(4.0));
    assert_eq!(styles[0].border_bottom_color, color::palette::css::GREEN);
    assert_eq!(styles[0].border_left_color, color::palette::css::GREEN);
    assert_eq!(styles[0].border_right_color, color::palette::css::GREEN);
    assert_eq!(styles[0].border_top_color, color::palette::css::GREEN);
    assert_eq!(styles[1].border_bottom_width, Style::default().border_bottom_width);
    assert_eq!(styles[1].border_left_width, Style::default().border_left_width);
    assert_eq!(styles[1].border_right_width, Style::default().border_right_width);
    assert_eq!(styles[1].border_top_width, Style::default().border_top_width);
    assert_eq!(styles[1].border_bottom_color, Style::default().border_bottom_color);
    assert_eq!(styles[1].border_left_color, Style::default().border_left_color);
    assert_eq!(styles[1].border_right_color, Style::default().border_right_color);
    assert_eq!(styles[1].border_top_color, Style::default().border_top_color);
}

use super::util::*;

#[test]
fn stylesheet_to_string() {
    let sheet_one = Stylesheet::from_str(include_str!("./test.css")).unwrap();
    let sheet_two = Stylesheet::from_str(&sheet_one.to_string()).unwrap();
    assert_eq!(sheet_one, sheet_two);
}

macro_rules! generate_css_test {
    (@color, $inherited:expr, $property_name:expr, $rust_property:ident) => {
        #[test]
        fn $rust_property() {
            // Auto (no effect)
            let styles = apply_css_to_tree(concat!(".root { ", $property_name, ": auto; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, Style::default().$rust_property);

            // Initial
            let styles =
                apply_css_to_tree(concat!(".child { ", $property_name, ": rgb(0, 128, 0); } .right { ", $property_name, ": initial; }"), two_child_tree);
            assert_eq!(styles[1].$rust_property, color::palette::css::GREEN);
            assert_eq!(styles[2].$rust_property, Style::default().$rust_property);

            // Inherit
            let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": #0FF; } .child { ", $property_name, ": inherit; }"), one_child_tree);
            assert_eq!(styles[0].$rust_property, color::parse_color("#0FF").unwrap().to_alpha_color::<Srgb>());
            assert_eq!(styles[1].$rust_property, color::parse_color("#0FF").unwrap().to_alpha_color::<Srgb>());

            // Default behavior
            if $inherited {
                let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": green; }"), one_child_tree);
                assert_eq!(styles[0].$rust_property, color::palette::css::GREEN);
                assert_eq!(styles[1].$rust_property, color::palette::css::GREEN);
            } else {
                let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": green; }"), one_child_tree);
                assert_eq!(styles[0].$rust_property, color::palette::css::GREEN);
                assert_eq!(styles[1].$rust_property, Style::default().$rust_property);
            }

            // Current color
            let styles = apply_css_to_tree(concat!(".parent { color: green; } .child { ", $property_name, ": currentcolor; }"), one_child_tree);
            assert_eq!(styles[1].$rust_property, color::palette::css::GREEN);

            // same node gets currentcolor first, then color later in cascade
            let styles = apply_css_to_tree(concat!(".root { ", $property_name, ": currentcolor; } .root { color: red; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, color::palette::css::RED);

            // intermediate color overridden later
            let styles = apply_css_to_tree(concat!(".root { color: green; ", $property_name, ": currentcolor; } .root { color: red; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, color::palette::css::RED);
        }
    };
    (@f32, $inherited:expr, $property_name:expr, $rust_property:ident) => {
        #[test]
        fn $rust_property() {
            // None
            let styles = apply_css_to_tree(concat!(".root { ", $property_name, ": none; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, Style::default().$rust_property);

            // Normal Use
            let styles = apply_css_to_tree(concat!(".root { ", $property_name, ": 10px; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, 10.0);

            // Initial
            let styles = apply_css_to_tree(concat!(".child { ", $property_name, ": 10px; } .right { ", $property_name, ": initial; }"), two_child_tree);
            assert_eq!(styles[1].$rust_property, 10.0);
            assert_eq!(styles[2].$rust_property, Style::default().$rust_property);

            // Inherit
            let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 10px; } .child { ", $property_name, ": inherit; }"), one_child_tree);
            assert_eq!(styles[0].$rust_property, 10.0);
            assert_eq!(styles[1].$rust_property, 10.0);

            // Default behavior
            if $inherited {
                let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 10px; }"), one_child_tree);
                assert_eq!(styles[0].$rust_property, 10.0);
                assert_eq!(styles[1].$rust_property, 10.0);
            } else {
                let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 10px; }"), one_child_tree);
                assert_eq!(styles[0].$rust_property, 10.0);
                assert_eq!(styles[1].$rust_property, Style::default().$rust_property);
            }
        }
    };
    (@unit, $inherited:expr, $property_name:expr, $rust_property:ident) => {
        #[test]
        fn $rust_property() {
            // None
            let styles = apply_css_to_tree(concat!(".root { ", $property_name, ": none; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, Style::default().$rust_property);

            // Normal Use
            let styles = apply_css_to_tree(concat!(".root { ", $property_name, ": 2s; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, Unit::Stretch(2.0));

            // Initial
            let styles = apply_css_to_tree(concat!(".child { ", $property_name, ": 2s; } .right { ", $property_name, ": initial; }"), two_child_tree);
            assert_eq!(styles[1].$rust_property, Unit::Stretch(2.0));
            assert_eq!(styles[2].$rust_property, Style::default().$rust_property);

            // Inherit
            let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 2s; } .child { ", $property_name, ": inherit; }"), one_child_tree);
            assert_eq!(styles[0].$rust_property, Unit::Stretch(2.0));
            assert_eq!(styles[1].$rust_property, Unit::Stretch(2.0));

            // Default behavior
            if $inherited {
                let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 2s; }"), one_child_tree);
                assert_eq!(styles[0].$rust_property, Unit::Stretch(2.0));
                assert_eq!(styles[1].$rust_property, Unit::Stretch(2.0));
            } else {
                let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 2s; }"), one_child_tree);
                assert_eq!(styles[0].$rust_property, Unit::Stretch(2.0));
                assert_eq!(styles[1].$rust_property, Style::default().$rust_property);
            }
        }
    };
    (@unit_opt, $inherited:expr, $property_name:expr, $rust_property:ident) => {
        #[test]
        fn $rust_property() {
            // None
            let styles = apply_css_to_tree(concat!(".root { ", $property_name, ": none; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, None);

            // Normal Use
            let styles = apply_css_to_tree(concat!(".root { ", $property_name, ": 2s; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, Some(Unit::Stretch(2.0)));

            // Initial
            let styles = apply_css_to_tree(concat!(".child { ", $property_name, ": 2s; } .right { ", $property_name, ": initial; }"), two_child_tree);
            assert_eq!(styles[1].$rust_property, Some(Unit::Stretch(2.0)));
            assert_eq!(styles[2].$rust_property, Style::default().$rust_property);

            // Inherit
            let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 2s; } .child { ", $property_name, ": inherit; }"), one_child_tree);
            assert_eq!(styles[0].$rust_property, Some(Unit::Stretch(2.0)));
            assert_eq!(styles[1].$rust_property, Some(Unit::Stretch(2.0)));

            // Default behavior
            if $inherited {
                let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 2s; }"), one_child_tree);
                assert_eq!(styles[0].$rust_property, Some(Unit::Stretch(2.0)));
                assert_eq!(styles[1].$rust_property, Some(Unit::Stretch(2.0)));
            } else {
                let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 2s; }"), one_child_tree);
                assert_eq!(styles[0].$rust_property, Some(Unit::Stretch(2.0)));
                assert_eq!(styles[1].$rust_property, None);
            }
        }
    };
    (@length, $inherited:expr, $property_name:expr, $rust_property:ident) => {
        #[test]
        fn $rust_property() {
            // None
            let styles = apply_css_to_tree(concat!(".root { ", $property_name, ": none; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, Style::default().$rust_property);

            // Normal Use
            let styles = apply_css_to_tree(concat!(".root { ", $property_name, ": 10px; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, Length::Px(10.0));

            // Initial
            let styles = apply_css_to_tree(concat!(".child { ", $property_name, ": 10px; } .right { ", $property_name, ": initial; }"), two_child_tree);
            assert_eq!(styles[1].$rust_property, Length::Px(10.0));
            assert_eq!(styles[2].$rust_property, Style::default().$rust_property);

            // Inherit
            let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 10px; } .child { ", $property_name, ": inherit; }"), one_child_tree);
            assert_eq!(styles[0].$rust_property, Length::Px(10.0));
            assert_eq!(styles[1].$rust_property, Length::Px(10.0));

            // Default behavior
            if $inherited {
                let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 10px; }"), one_child_tree);
                assert_eq!(styles[0].$rust_property, Length::Px(10.0));
                assert_eq!(styles[1].$rust_property, Length::Px(10.0));
            } else {
                let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 10px; }"), one_child_tree);
                assert_eq!(styles[0].$rust_property, Length::Px(10.0));
                assert_eq!(styles[1].$rust_property, Style::default().$rust_property);
            }
        }
    };
    (@length_opt, $inherited:expr, $property_name:expr, $rust_property:ident) => {
        #[test]
        fn $rust_property() {
            // None
            let styles = apply_css_to_tree(concat!(".root { ", $property_name, ": none; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, None);

            // Normal Use
            let styles = apply_css_to_tree(concat!(".root { ", $property_name, ": 10px; }"), single_node_tree);
            assert_eq!(styles[0].$rust_property, Some(Length::Px(10.0)));

            // Initial
            let styles = apply_css_to_tree(concat!(".child { ", $property_name, ": 10px; } .right { ", $property_name, ": initial; }"), two_child_tree);
            assert_eq!(styles[1].$rust_property, Some(Length::Px(10.0)));
            assert_eq!(styles[2].$rust_property, Style::default().$rust_property);

            // Inherit
            let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 10px; } .child { ", $property_name, ": inherit; }"), one_child_tree);
            assert_eq!(styles[0].$rust_property, Some(Length::Px(10.0)));
            assert_eq!(styles[1].$rust_property, Some(Length::Px(10.0)));

            // Default behavior
            if $inherited {
                let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 10px; }"), one_child_tree);
                assert_eq!(styles[0].$rust_property, Some(Length::Px(10.0)));
                assert_eq!(styles[1].$rust_property, Some(Length::Px(10.0)));
            } else {
                let styles = apply_css_to_tree(concat!(".parent { ", $property_name, ": 10px; }"), one_child_tree);
                assert_eq!(styles[0].$rust_property, Some(Length::Px(10.0)));
                assert_eq!(styles[1].$rust_property, Style::default().$rust_property);
            }
        }
    };
}

generate_css_test!(@color, false, "background-color", background_color);
generate_css_test!(@color, false, "border-bottom-color", border_bottom_color);
generate_css_test!(@length, false, "border-bottom-left-radius", border_bottom_left_radius);
generate_css_test!(@length, false, "border-bottom-right-radius", border_bottom_right_radius);
generate_css_test!(@length, false, "border-bottom-width", border_bottom_width);
generate_css_test!(@color, false, "border-left-color", border_left_color);
generate_css_test!(@length, false, "border-left-width", border_left_width);
generate_css_test!(@color, false, "border-right-color", border_right_color);
generate_css_test!(@length, false, "border-right-width", border_right_width);
generate_css_test!(@color, false, "border-top-color", border_top_color);
generate_css_test!(@length, false, "border-top-left-radius", border_top_left_radius);
generate_css_test!(@length, false, "border-top-right-radius", border_top_right_radius);
generate_css_test!(@length, false, "border-top-width", border_top_width);
generate_css_test!(@unit, false, "bottom", bottom);
generate_css_test!(@unit, false, "child-between", child_between);
generate_css_test!(@unit, false, "child-bottom", child_bottom);
generate_css_test!(@unit, false, "child-left", child_left);
generate_css_test!(@unit, false, "child-right", child_right);
generate_css_test!(@unit, false, "child-top", child_top);
generate_css_test!(@color, true, "color", color);
generate_css_test!(@length, false, "flex-basis", flex_basis);
generate_css_test!(@f32, true, "font-size", font_size);
generate_css_test!(@unit, false, "height", height);
generate_css_test!(@unit, false, "left", left);
generate_css_test!(@unit_opt, true, "letter-spacing", letter_spacing);
generate_css_test!(@unit, true, "line-height", line_height);
generate_css_test!(@length_opt, false, "max-bottom", max_bottom);
generate_css_test!(@length_opt, false, "max-child-between", max_child_between);
generate_css_test!(@length_opt, false, "max-child-bottom", max_child_bottom);
generate_css_test!(@length_opt, false, "max-child-left", max_child_left);
generate_css_test!(@length_opt, false, "max-child-right", max_child_right);
generate_css_test!(@length_opt, false, "max-child-top", max_child_top);
generate_css_test!(@length_opt, false, "max-height", max_height);
generate_css_test!(@length_opt, false, "max-left", max_left);
generate_css_test!(@length_opt, false, "max-right", max_right);
generate_css_test!(@length_opt, false, "max-top", max_top);
generate_css_test!(@length_opt, false, "max-width", max_width);
generate_css_test!(@length_opt, false, "min-bottom", min_bottom);
generate_css_test!(@length_opt, false, "min-child-between", min_child_between);
generate_css_test!(@length_opt, false, "min-child-bottom", min_child_bottom);
generate_css_test!(@length_opt, false, "min-child-left", min_child_left);
generate_css_test!(@length_opt, false, "min-child-right", min_child_right);
generate_css_test!(@length_opt, false, "min-child-top", min_child_top);
generate_css_test!(@length_opt, false, "min-height", min_height);
generate_css_test!(@length_opt, false, "min-left", min_left);
generate_css_test!(@length_opt, false, "min-right", min_right);
generate_css_test!(@length_opt, false, "min-top", min_top);
generate_css_test!(@length_opt, false, "min-width", min_width);
generate_css_test!(@color, false, "outline-color", outline_color);
generate_css_test!(@length, false, "outline-offset", outline_offset);
generate_css_test!(@length, false, "outline-width", outline_width);
generate_css_test!(@unit, false, "right", right);
generate_css_test!(@unit, false, "top", top);
generate_css_test!(@unit, false, "width", width);
generate_css_test!(@unit_opt, true, "word-spacing", word_spacing);

#[test]
fn display() {
    // None
    let styles = apply_css_to_tree(".root { display: none; }", single_node_tree);
    assert_eq!(styles[0].display, None);

    // Normal Use
    let styles = apply_css_to_tree(".root { display: column; }", single_node_tree);
    assert_eq!(styles[0].display, Some(Direction::Column));

    // Initial
    let styles = apply_css_to_tree(".child { display: row; } .right { display: initial; }", two_child_tree);
    assert_eq!(styles[1].display, Some(Direction::Row));
    assert_eq!(styles[2].display, Style::default().display);

    // Inherit
    let styles = apply_css_to_tree(".parent { display: row-reverse; } .child { display: inherit; }", one_child_tree);
    assert_eq!(styles[0].display, Some(Direction::RowReverse));
    assert_eq!(styles[1].display, Some(Direction::RowReverse));

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { display: column-reverse; }", one_child_tree);
    assert_eq!(styles[0].display, Some(Direction::ColumnReverse));
    assert_eq!(styles[1].display, Style::default().display);
}

#[test]
fn opacity() {
    let styles = apply_css_to_tree(".root { opacity: none; }", single_node_tree);
    assert_eq!(styles[0].opacity, Style::default().opacity);

    let styles = apply_css_to_tree(".root { opacity: 10%; }", single_node_tree);
    assert_eq!(styles[0].opacity, 0.1);

    let styles = apply_css_to_tree(".child { opacity: 10%; } .right { opacity: initial; }", two_child_tree);
    assert_eq!(styles[1].opacity, 0.1);
    assert_eq!(styles[2].opacity, Style::default().opacity);

    let styles = apply_css_to_tree(".parent { opacity: 10%; } .child { opacity: inherit; }", one_child_tree);
    assert_eq!(styles[0].opacity, 0.1);
    assert_eq!(styles[1].opacity, 0.1);

    let styles = apply_css_to_tree(".parent { opacity: 10%; }", one_child_tree);
    assert_eq!(styles[0].opacity, 0.1);
    assert_eq!(styles[1].opacity, Style::default().opacity);
}

#[test]
fn outline() {
    // Normal use
    let styles = apply_css_to_tree(".root { outline: 2px rgb(255, 0, 0); }", single_node_tree);
    assert_eq!(styles[0].outline_width, Length::Px(2.0));
    assert_eq!(styles[0].outline_color, color::palette::css::RED);

    // Initial
    let styles = apply_css_to_tree(".parent { outline: 3px #00F; } .child { outline: initial; }", one_child_tree);
    assert_eq!(styles[0].outline_width, Length::Px(3.0));
    assert_eq!(styles[0].outline_color, color::palette::css::BLUE);
    assert_eq!(styles[1].outline_width, Style::default().outline_width);
    assert_eq!(styles[1].outline_color, Style::default().outline_color);

    // Inherit
    let styles = apply_css_to_tree(".parent { outline: 3px green; } .child { outline: inherit; }", one_child_tree);
    assert_eq!(styles[0].outline_width, Length::Px(3.0));
    assert_eq!(styles[0].outline_color, color::palette::css::GREEN);
    assert_eq!(styles[1].outline_width, Length::Px(3.0));
    assert_eq!(styles[1].outline_color, color::palette::css::GREEN);

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { outline: 4px purple; }", one_child_tree);
    assert_eq!(styles[0].outline_width, Length::Px(4.0));
    assert_eq!(styles[0].outline_color, color::palette::css::PURPLE);
    assert_eq!(styles[1].outline_width, Style::default().outline_width);
    assert_eq!(styles[1].outline_color, Style::default().outline_color);
}

#[test]
fn outline_reject() {
    fn reject(value: &str) {
        let css = format!(".root {{ outline: {}; }}", value);
        let styles = apply_css_to_tree(&css, single_node_tree);
        assert_eq!(styles[0].outline_width, Style::default().outline_width, "expected parser to reject outline: {} (outline_width)", value);
        assert_eq!(styles[0].outline_color, Style::default().outline_color, "expected parser to reject outline: {} (outline_color)", value);
    }

    for value in [
        // Empty / missing pieces
        "",          // empty value
        "()",        // unexpected empty parens
        "none red",  // trailing token after keyword
        "2px none",  // unsupported keyword in place of color
        "2px , red", // unexpected comma
        "2px red ,", // trailing comma
        // Unsupported outline-style tokens
        "2px dotted red", // unsupported outline-style keyword
        "2px dashed red", // unsupported outline-style keyword
        // Length malformed
        "-1px red",    // negative width
        "0.5 red",     // unitless width
        "2 px red",    // split length token
        "2p x red",    // invalid length token
        "2s red",      // invalid unit
        "2% red",      // percent length not allowed here
        "2px 3px red", // extra length component
        "2px 3 red",   // extra token
        // Color malformed / unsupported
        "2px #12",                         // invalid hex length
        "2px #GGG",                        // invalid hex digits
        "2px rgb(255, 0)",                 // wrong rgb arity
        "2px url(test.png)",               // not a color
        "2px linear-gradient(#fff, #000)", // not a color
        "2px foo",                         // unknown identifier as color
        // Ordering / separators
        "red, 2px",           // comma separator not allowed
        "2px red extra",      // trailing token
        "2px red !important", // contains !important
        "2px (red)",          // unexpected parentheses
    ] {
        reject(value);
    }
}

#[test]
fn position() {
    // Parent Directed
    let styles = apply_css_to_tree(".root { position: parent-directed; }", single_node_tree);
    assert_eq!(styles[0].position, Position::ParentDirected);

    // Self Directed
    let styles = apply_css_to_tree(".root { position: self-directed; }", single_node_tree);
    assert_eq!(styles[0].position, Position::SelfDirected);

    // Fixed
    let styles = apply_css_to_tree(".root { position: fixed; }", single_node_tree);
    assert_eq!(styles[0].position, Position::Fixed);

    // Initial
    let styles = apply_css_to_tree(".child { position: fixed; } .right { position: initial; }", two_child_tree);
    assert_eq!(styles[1].position, Position::Fixed);
    assert_eq!(styles[2].position, Style::default().position);

    // Inherit
    let styles = apply_css_to_tree(".parent { position: fixed; } .child { position: inherit; }", one_child_tree);
    assert_eq!(styles[0].position, Position::Fixed);
    assert_eq!(styles[1].position, Position::Fixed);

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { position: fixed; }", one_child_tree);
    assert_eq!(styles[0].position, Position::Fixed);
    assert_eq!(styles[1].position, Style::default().position);
}

#[test]
fn space() {
    // Normal Use
    let styles = apply_css_to_tree(".root { space: 2s 3s 4s 5s; }", single_node_tree);
    assert_eq!(styles[0].top, Unit::Stretch(2.0));
    assert_eq!(styles[0].right, Unit::Stretch(3.0));
    assert_eq!(styles[0].bottom, Unit::Stretch(4.0));
    assert_eq!(styles[0].left, Unit::Stretch(5.0));

    // One Value
    let styles = apply_css_to_tree(".root { space: 2s; }", single_node_tree);
    assert_eq!(styles[0].top, Unit::Stretch(2.0));
    assert_eq!(styles[0].right, Unit::Stretch(2.0));
    assert_eq!(styles[0].bottom, Unit::Stretch(2.0));
    assert_eq!(styles[0].left, Unit::Stretch(2.0));

    // Initial
    let styles = apply_css_to_tree(".parent { space: 3s 4s 5s 6s; } .child { space: initial; }", one_child_tree);
    assert_eq!(styles[0].top, Unit::Stretch(3.0));
    assert_eq!(styles[0].right, Unit::Stretch(4.0));
    assert_eq!(styles[0].bottom, Unit::Stretch(5.0));
    assert_eq!(styles[0].left, Unit::Stretch(6.0));
    assert_eq!(styles[1].top, Style::default().top);
    assert_eq!(styles[1].right, Style::default().right);
    assert_eq!(styles[1].bottom, Style::default().bottom);
    assert_eq!(styles[1].left, Style::default().left);

    // Inherit
    let styles = apply_css_to_tree(".parent { space: 3s 4s 5s 6s; } .child { space: inherit; }", one_child_tree);
    assert_eq!(styles[0].top, Unit::Stretch(3.0));
    assert_eq!(styles[0].right, Unit::Stretch(4.0));
    assert_eq!(styles[0].bottom, Unit::Stretch(5.0));
    assert_eq!(styles[0].left, Unit::Stretch(6.0));
    assert_eq!(styles[1].top, Unit::Stretch(3.0));
    assert_eq!(styles[1].right, Unit::Stretch(4.0));
    assert_eq!(styles[1].bottom, Unit::Stretch(5.0));
    assert_eq!(styles[1].left, Unit::Stretch(6.0));

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { space: 4s 5s 6s 7s; }", one_child_tree);
    assert_eq!(styles[0].top, Unit::Stretch(4.0));
    assert_eq!(styles[0].right, Unit::Stretch(5.0));
    assert_eq!(styles[0].bottom, Unit::Stretch(6.0));
    assert_eq!(styles[0].left, Unit::Stretch(7.0));
    assert_eq!(styles[1].top, Style::default().top);
    assert_eq!(styles[1].right, Style::default().right);
    assert_eq!(styles[1].bottom, Style::default().bottom);
    assert_eq!(styles[1].left, Style::default().left);
}

#[test]
fn child_space() {
    // Normal Use
    let styles = apply_css_to_tree(".root { child-space: 2s 3s 4s 5s; }", single_node_tree);
    assert_eq!(styles[0].child_top, Unit::Stretch(2.0));
    assert_eq!(styles[0].child_right, Unit::Stretch(3.0));
    assert_eq!(styles[0].child_bottom, Unit::Stretch(4.0));
    assert_eq!(styles[0].child_left, Unit::Stretch(5.0));

    // One Value
    let styles = apply_css_to_tree(".root { child-space: 2s; }", single_node_tree);
    assert_eq!(styles[0].child_top, Unit::Stretch(2.0));
    assert_eq!(styles[0].child_right, Unit::Stretch(2.0));
    assert_eq!(styles[0].child_bottom, Unit::Stretch(2.0));
    assert_eq!(styles[0].child_left, Unit::Stretch(2.0));

    // Initial
    let styles =
        apply_css_to_tree(".parent { child-space: 3s 4s 5s 6s; } .child { child-space: initial; }", one_child_tree);
    assert_eq!(styles[0].child_top, Unit::Stretch(3.0));
    assert_eq!(styles[0].child_right, Unit::Stretch(4.0));
    assert_eq!(styles[0].child_bottom, Unit::Stretch(5.0));
    assert_eq!(styles[0].child_left, Unit::Stretch(6.0));
    assert_eq!(styles[1].child_top, Style::default().child_top);
    assert_eq!(styles[1].child_right, Style::default().child_right);
    assert_eq!(styles[1].child_bottom, Style::default().child_bottom);
    assert_eq!(styles[1].child_left, Style::default().child_left);

    // Inherit
    let styles =
        apply_css_to_tree(".parent { child-space: 3s 4s 5s 6s; } .child { child-space: inherit; }", one_child_tree);
    assert_eq!(styles[0].child_top, Unit::Stretch(3.0));
    assert_eq!(styles[0].child_right, Unit::Stretch(4.0));
    assert_eq!(styles[0].child_bottom, Unit::Stretch(5.0));
    assert_eq!(styles[0].child_left, Unit::Stretch(6.0));
    assert_eq!(styles[1].child_top, Unit::Stretch(3.0));
    assert_eq!(styles[1].child_right, Unit::Stretch(4.0));
    assert_eq!(styles[1].child_bottom, Unit::Stretch(5.0));
    assert_eq!(styles[1].child_left, Unit::Stretch(6.0));

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { child-space: 4s 5s 6s 7s; }", one_child_tree);
    assert_eq!(styles[0].child_top, Unit::Stretch(4.0));
    assert_eq!(styles[0].child_right, Unit::Stretch(5.0));
    assert_eq!(styles[0].child_bottom, Unit::Stretch(6.0));
    assert_eq!(styles[0].child_left, Unit::Stretch(7.0));
    assert_eq!(styles[1].child_top, Style::default().child_top);
    assert_eq!(styles[1].child_right, Style::default().child_right);
    assert_eq!(styles[1].child_bottom, Style::default().child_bottom);
    assert_eq!(styles[1].child_left, Style::default().child_left);
}

#[test]
fn z_index() {
    // Auto
    let styles = apply_css_to_tree(".root { z-index: auto; }", single_node_tree);
    assert_eq!(styles[0].z_index, Style::default().z_index);

    // Specific Value
    let styles = apply_css_to_tree(".root { z-index: 10; }", single_node_tree);
    assert_eq!(styles[0].z_index, 10);

    // Initial
    let styles = apply_css_to_tree(".child { z-index: 10; } .right { z-index: initial; }", two_child_tree);
    assert_eq!(styles[1].z_index, 10);
    assert_eq!(styles[2].z_index, Style::default().z_index);

    // Inherit
    let styles = apply_css_to_tree(".parent { z-index: 10; } .child { z-index: inherit; }", one_child_tree);
    assert_eq!(styles[0].z_index, 10);
    assert_eq!(styles[1].z_index, 10);

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { z-index: 10; }", one_child_tree);
    assert_eq!(styles[0].z_index, 10);
    assert_eq!(styles[1].z_index, Style::default().z_index);
}

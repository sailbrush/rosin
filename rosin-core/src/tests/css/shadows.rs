use super::util::*;

#[test]
fn box_shadow() {
    let shadow: Option<Arc<[BoxShadow]>> = Some(Arc::new([BoxShadow {
        offset_x: Length::Px(5.0),
        offset_y: Length::Px(10.0),
        blur: Length::Px(15.0),
        spread: Length::Px(20.0),
        color: Some(color::palette::css::BLUE),
        inset: false,
    }]));

    let shadow_current_color: Option<Arc<[BoxShadow]>> = Some(Arc::new([BoxShadow {
        offset_x: Length::Px(5.0),
        offset_y: Length::Px(10.0),
        blur: Length::Px(15.0),
        spread: Length::Px(20.0),
        color: None,
        inset: true,
    }]));

    // None
    let styles = apply_css_to_tree(".root { box-shadow: none; }", single_node_tree);
    assert_eq!(styles[0].box_shadow, Style::default().box_shadow);

    // Normal Use
    let styles = apply_css_to_tree(".root { box-shadow: 5px 10px 15px 20px #00F; }", single_node_tree);
    assert_eq!(styles[0].box_shadow, shadow);

    // Initial
    let styles = apply_css_to_tree(".child { box-shadow: 5px 10px 15px 20px rgb(0, 0, 255); } .right { box-shadow: initial; }", two_child_tree);
    assert_eq!(styles[1].box_shadow, shadow);
    assert_eq!(styles[2].box_shadow, Style::default().box_shadow);

    // Inherit
    let styles = apply_css_to_tree(".parent { box-shadow: 5px 10px 15px 20px blue; } .child { box-shadow: inherit; }", one_child_tree);
    assert_eq!(styles[0].box_shadow, shadow);
    assert_eq!(styles[1].box_shadow, shadow);

    // Default (uninherited)
    let styles = apply_css_to_tree(".parent { box-shadow: 5px 10px 15px 20px blue; }", one_child_tree);
    assert_eq!(styles[0].box_shadow, shadow);
    assert_eq!(styles[1].box_shadow, Style::default().box_shadow);

    // Current color
    let styles = apply_css_to_tree(".root { box-shadow: 5px 10px 15px 20px currentcolor inset; }", single_node_tree);
    assert_eq!(styles[0].box_shadow, shadow_current_color);
}

#[test]
fn box_shadow_reject() {
    fn reject(value: &str) {
        let css = format!(".root {{ box-shadow: {}; }}", value);
        let styles = apply_css_to_tree(&css, single_node_tree);
        assert_eq!(styles[0].box_shadow, Style::default().box_shadow, "expected parser to reject box-shadow: {}", value);
    }

    for value in [
        // Not valid
        "",                                               // empty value
        "()",                                             // unexpected empty parens
        "auto",                                           // unsupported keyword
        "foo",                                            // unknown identifier
        "#fff",                                           // color alone (missing lengths)
        "5px",                                            // too few components
        "5px 10px 15px 20px blue extra",                  // trailing token
        "inset",                                          // keyword alone (missing lengths)
        "inset extra",                                    // keyword + trailing token
        "5px 10px 15px 20px blue,",                       // trailing comma
        ",5px 10px 15px 20px blue",                       // leading comma
        "5px 10px 15px 20px blue,,",                      // empty list item
        "5px 10px 15px 20px blue, , 1px 2px 3px 4px red", // empty list item
        "5px 10px 15px 20px blue 1px 2px 3px 4px red",    // missing comma between shadows
        // Length malformed (unitless / wrong units / split tokens)
        "5 10px 15px 20px blue",    // unitless offset-x
        "5px 10 15px 20px blue",    // unitless offset-y
        "5px 10px 15 20px blue",    // unitless blur
        "5px 10px 15px 20 blue",    // unitless spread
        "5 px 10px 15px 20px blue", // split length token
        "5px 10 px 15px 20px blue", // split length token
        "5px 10px 15 px 20px blue", // split length token
        "5px 10px 15px 20 px blue", // split length token
        "5% 10px 15px 20px blue",   // percent length not allowed here
        "5px 10% 15px 20px blue",   // percent length not allowed here
        "5px 10px 15% 20px blue",   // percent length not allowed here
        "5px 10px 15px 20% blue",   // percent length not allowed here
        "5s 10px 15px 20px blue",   // invalid unit
        "5px 10s 15px 20px blue",   // invalid unit
        "5px 10px 15s 20px blue",   // invalid unit
        "5px 10px 15px 20s blue",   // invalid unit
        // Negative constraints
        "5px 10px -1px 20px blue", // negative blur
        // Weird separators / unexpected tokens
        "5px, 10px, 15px, 20px, blue", // commas between components
        "5px 10px 15px 20px / blue",   // unexpected '/'
        "5px 10px 15px 20px blue / 1", // unexpected '/'
        // Color malformed / unsupported
        "5px 10px 15px 20px #12",                        // invalid hex length
        "5px 10px 15px 20px #GGG",                       // invalid hex digits
        "5px 10px 15px 20px rgb(255,0)",                 // wrong rgb arity
        "5px 10px 15px 20px rgb()",                      // empty rgb()
        "5px 10px 15px 20px url(x)",                     // not a color
        "5px 10px 15px 20px linear-gradient(#fff,#000)", // not a color
        "5px 10px 15px 20px 123",                        // number is not a color
        "5px 10px 15px 20px blue,",                      // stray comma after color
        // inset keyword placement / duplication
        "5px 10px 15px 20px blue inset inset", // duplicate inset
        "5px 10px 15px 20px blue insett",      // unknown keyword
        // invalid parentheses
        "5px (10px) 15px 20px blue",               // unexpected parentheses
        "5px 10px (15px 20px blue)",               // unexpected parentheses
        "5px 10px 15px 20px (blue",                // unclosed '('
        "5px 10px 15px 20px blue)",                // unmatched ')'
        "5px 10px 15px 20px (currentcolor) inset", // unexpected parentheses
        // Trailing tokens
        "5px 10px 15px 20px blue !important", // contains !important
    ] {
        reject(value);
    }
}

#[test]
fn text_shadow() {
    let shadow: Option<Arc<[TextShadow]>> = Some(Arc::new([TextShadow {
        offset_x: Length::Px(5.0),
        offset_y: Length::Px(10.0),
        blur: Length::Px(15.0),
        color: Some(color::palette::css::BLUE),
    }]));
    let shadow_current_color: Option<Arc<[TextShadow]>> = Some(Arc::new([TextShadow {
        offset_x: Length::Px(5.0),
        offset_y: Length::Px(10.0),
        blur: Length::Px(15.0),
        color: None,
    }]));

    // None
    let styles = apply_css_to_tree(".root { text-shadow: none; }", single_node_tree);
    assert_eq!(styles[0].text_shadow, Style::default().text_shadow);

    // Normal Use
    let styles = apply_css_to_tree(".root { text-shadow: 5px 10px 15px #00F; }", single_node_tree);
    assert_eq!(styles[0].text_shadow, shadow);

    // Initial
    let styles = apply_css_to_tree(".child { text-shadow: 5px 10px 15px rgb(0, 0, 255); } .right { text-shadow: initial; }", two_child_tree);
    assert_eq!(styles[1].text_shadow, shadow);
    assert_eq!(styles[2].text_shadow, Style::default().text_shadow);

    // Inherit
    let styles = apply_css_to_tree(".parent { text-shadow: 5px 10px 15px blue; } .child { text-shadow: inherit; }", one_child_tree);
    assert_eq!(styles[0].text_shadow, shadow);
    assert_eq!(styles[1].text_shadow, shadow);

    // Default (inherited)
    let styles = apply_css_to_tree(".parent { text-shadow: 5px 10px 15px blue; }", one_child_tree);
    assert_eq!(styles[0].text_shadow, shadow);
    assert_eq!(styles[1].text_shadow, shadow);

    // Current color
    let styles = apply_css_to_tree(".root { text-shadow: 5px 10px 15px currentcolor; }", single_node_tree);
    assert_eq!(styles[0].text_shadow, shadow_current_color);
}

#[test]
fn text_shadow_reject() {
    fn reject(value: &str) {
        let css = format!(".root {{ text-shadow: {}; }}", value);
        let styles = apply_css_to_tree(&css, single_node_tree);
        assert_eq!(styles[0].text_shadow, Style::default().text_shadow, "expected parser to reject text-shadow: {}", value);
    }

    for value in [
        // Not valid
        "",                    // empty value
        "()",                  // unexpected empty parens
        "auto",                // unsupported keyword
        "foo",                 // unknown identifier
        "#fff",                // color alone (missing lengths)
        "5px",                 // too few components
        "5px 10px blue extra", // trailing token
        // Length malformed
        "5 10px 15px blue",    // unitless offset-x
        "5px 10 15px blue",    // unitless offset-y
        "5px 10px 15 blue",    // unitless blur
        "5px 10px -1px blue",  // negative blur
        "5 px 10px 15px blue", // split length token
        "5px 10 px 15px blue", // split length token
        "5px 10px 15 px blue", // split length token
        "5% 10px 15px blue",   // percent length not allowed here
        "5px 10% 15px blue",   // percent length not allowed here
        "5px 10px 15% blue",   // percent length not allowed here
        "5s 10px 15px blue",   // invalid unit
        "5px 10s 15px blue",   // invalid unit
        "5px 10px 15s blue",   // invalid unit
        // Separators / unexpected tokens
        "5px, 10px, 15px, blue",  // commas between components
        "5px 10px 15px / blue",   // unexpected '/'
        "5px 10px 15px blue / 1", // unexpected '/'
        // Color malformed / unsupported
        "5px 10px 15px #12",                        // invalid hex length
        "5px 10px 15px #GGG",                       // invalid hex digits
        "5px 10px 15px rgb(255,0)",                 // wrong rgb arity
        "5px 10px 15px url(x)",                     // not a color
        "5px 10px 15px linear-gradient(#fff,#000)", // not a color
        "5px 10px 15px 123",                        // number is not a color
        "5px 10px 15px blue,",                      // stray comma after color
        // Arity / separators / list issues
        "5px 10px 15px blue,",                   // trailing comma
        ",5px 10px 15px blue",                   // leading comma
        "5px 10px 15px blue,,",                  // empty list item
        "5px 10px 15px blue, , 1px 2px 3px red", // empty list item
        "5px 10px 15px blue 1px 2px 3px red",    // missing comma between shadows
        "5px 10px 15px blue !important",         // contains !important
        // Invalid parentheses
        "5px (10px) 15px blue", // unexpected parentheses
        "5px 10px (15px blue)", // unexpected parentheses
        "5px 10px 15px (blue",  // unclosed '('
        "5px 10px 15px blue)",  // unmatched ')'
    ] {
        reject(value);
    }
}

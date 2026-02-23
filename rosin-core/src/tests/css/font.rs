use super::util::*;

#[test]
fn font_family() {
    // Specific Font
    let styles = apply_css_to_tree(".root { font-family: Arial, sans-serif; }", single_node_tree);
    assert_eq!(*styles[0].font_family.as_ref().unwrap(), "Arial, sans-serif".into());

    // Initial
    let styles = apply_css_to_tree(".child { font-family: Arial, sans-serif; } .right { font-family: initial; }", two_child_tree);
    assert_eq!(*styles[1].font_family.as_ref().unwrap(), "Arial, sans-serif".into());
    assert_eq!(styles[2].font_family, Style::default().font_family);

    // Inherit
    let styles = apply_css_to_tree(".parent { font-family: Arial, sans-serif; } .child { font-family: inherit; }", one_child_tree);
    assert_eq!(*styles[0].font_family.as_ref().unwrap(), "Arial, sans-serif".into());
    assert_eq!(*styles[1].font_family.as_ref().unwrap(), "Arial, sans-serif".into());

    // Default (inherited)
    let styles = apply_css_to_tree(".parent { font-family: Arial, sans-serif; }", one_child_tree);
    assert_eq!(*styles[0].font_family.as_ref().unwrap(), "Arial, sans-serif".into());
    assert_eq!(*styles[1].font_family.as_ref().unwrap(), "Arial, sans-serif".into());
}

#[test]
fn font_width() {
    // None
    let styles = apply_css_to_tree(".root { font-width: none; }", single_node_tree);
    assert_eq!(styles[0].font_width, Style::default().font_width);

    // Normal Use
    let styles = apply_css_to_tree(".root { font-width: 200%; }", single_node_tree);
    assert_eq!(styles[0].font_width, 2.0);

    // Initial
    let styles = apply_css_to_tree(".child { font-width: 200%; } .right { font-width: initial; }", two_child_tree);
    assert_eq!(styles[1].font_width, 2.0);
    assert_eq!(styles[2].font_width, Style::default().font_width);

    // Inherit
    let styles = apply_css_to_tree(".parent { font-width: 200%; } .child { font-width: inherit; }", one_child_tree);
    assert_eq!(styles[0].font_width, 2.0);
    assert_eq!(styles[1].font_width, 2.0);

    // Default (inherited)
    let styles = apply_css_to_tree(".parent { font-width: 200%; }", one_child_tree);
    assert_eq!(styles[0].font_width, 2.0);
    assert_eq!(styles[1].font_width, 2.0);
}

#[test]
fn font_style() {
    // Normal
    let styles = apply_css_to_tree(".root { font-style: normal; }", single_node_tree);
    assert_eq!(styles[0].font_style, parley::fontique::FontStyle::Normal);

    // Italic
    let styles = apply_css_to_tree(".root { font-style: italic; }", single_node_tree);
    assert_eq!(styles[0].font_style, parley::fontique::FontStyle::Italic);

    // Oblique
    let styles = apply_css_to_tree(".root { font-style: oblique; }", single_node_tree);
    assert_eq!(styles[0].font_style, parley::fontique::FontStyle::Oblique(None));

    // Oblique 45deg
    let styles = apply_css_to_tree(".root { font-style: oblique 45deg; }", single_node_tree);
    assert_eq!(styles[0].font_style, parley::fontique::FontStyle::Oblique(Some(45.0)));

    // Initial
    let styles = apply_css_to_tree(".child { font-style: italic; } .right { font-style: initial; }", two_child_tree);
    assert_eq!(styles[1].font_style, parley::fontique::FontStyle::Italic);
    assert_eq!(styles[2].font_style, Style::default().font_style);

    // Inherit
    let styles = apply_css_to_tree(".parent { font-style: italic; } .child { font-style: inherit; }", one_child_tree);
    assert_eq!(styles[0].font_style, parley::fontique::FontStyle::Italic);
    assert_eq!(styles[1].font_style, parley::fontique::FontStyle::Italic);

    // Default (inherited)
    let styles = apply_css_to_tree(".parent { font-style: italic; }", one_child_tree);
    assert_eq!(styles[0].font_style, parley::fontique::FontStyle::Italic);
    assert_eq!(styles[1].font_style, parley::fontique::FontStyle::Italic);
}

#[test]
fn font_weight() {
    // Normal
    let styles = apply_css_to_tree(".root { font-weight: normal; }", single_node_tree);
    assert_eq!(styles[0].font_weight, parley::fontique::FontWeight::NORMAL.value());

    // Bold
    let styles = apply_css_to_tree(".root { font-weight: bold; }", single_node_tree);
    assert_eq!(styles[0].font_weight, parley::fontique::FontWeight::BOLD.value());

    // Numeric
    let styles = apply_css_to_tree(".root { font-weight: 750; }", single_node_tree);
    assert_eq!(styles[0].font_weight, 750.0);

    // Initial
    let styles = apply_css_to_tree(".child { font-weight: bold; } .right { font-weight: initial; }", two_child_tree);
    assert_eq!(styles[1].font_weight, parley::fontique::FontWeight::BOLD.value());
    assert_eq!(styles[2].font_weight, Style::default().font_weight);

    // Inherit
    let styles = apply_css_to_tree(".parent { font-weight: bold; } .child { font-weight: inherit; }", one_child_tree);
    assert_eq!(styles[0].font_weight, parley::fontique::FontWeight::BOLD.value());
    assert_eq!(styles[1].font_weight, parley::fontique::FontWeight::BOLD.value());

    // Default (inherited)
    let styles = apply_css_to_tree(".parent { font-weight: bold; }", one_child_tree);
    assert_eq!(styles[0].font_weight, parley::fontique::FontWeight::BOLD.value());
    assert_eq!(styles[1].font_weight, parley::fontique::FontWeight::BOLD.value());
}

#[test]
fn font() {
    // Single property
    let styles = apply_css_to_tree(".root { font: 50px Arial; }", single_node_tree);
    assert_eq!(styles[0].font_size, 50.0);
    assert_eq!(styles[0].font_style, Style::default().font_style);
    assert_eq!(styles[0].font_weight, Style::default().font_weight);
    assert_eq!(*styles[0].font_family.as_ref().unwrap(), "Arial".into());

    // Multiple properties
    let styles = apply_css_to_tree(".root { font: italic bold 50px Arial, sans-serif; }", single_node_tree);
    assert_eq!(styles[0].font_size, 50.0);
    assert_eq!(styles[0].font_style, parley::fontique::FontStyle::Italic);
    assert_eq!(styles[0].font_weight, parley::fontique::FontWeight::BOLD.value());
    assert_eq!(*styles[0].font_family.as_ref().unwrap(), "Arial, sans-serif".into());

    // Inherit
    let styles = apply_css_to_tree(".parent { font: italic bold 50px Arial, sans-serif; } .child { font: inherit; }", one_child_tree);
    assert_eq!(styles[0].font_size, 50.0);
    assert_eq!(styles[0].font_style, parley::fontique::FontStyle::Italic);
    assert_eq!(styles[0].font_weight, parley::fontique::FontWeight::BOLD.value());
    assert_eq!(*styles[0].font_family.as_ref().unwrap(), "Arial, sans-serif".into());
    assert_eq!(styles[1].font_size, 50.0);
    assert_eq!(styles[1].font_style, parley::fontique::FontStyle::Italic);
    assert_eq!(styles[1].font_weight, parley::fontique::FontWeight::BOLD.value());
    assert_eq!(*styles[1].font_family.as_ref().unwrap(), "Arial, sans-serif".into());

    // Initial
    let styles = apply_css_to_tree(".child { font: italic bold 50px Arial, sans-serif; } .right { font: initial; }", two_child_tree);
    assert_eq!(styles[1].font_size, 50.0);
    assert_eq!(styles[1].font_style, parley::fontique::FontStyle::Italic);
    assert_eq!(styles[1].font_weight, parley::fontique::FontWeight::BOLD.value());
    assert_eq!(*styles[1].font_family.as_ref().unwrap(), "Arial, sans-serif".into());
    assert_eq!(styles[2].font_size, Style::default().font_size);
    assert_eq!(styles[2].font_style, Style::default().font_style);
    assert_eq!(styles[2].font_weight, Style::default().font_weight);
    assert_eq!(styles[2].font_family, Style::default().font_family);

    // Shorthand with line-height
    let styles = apply_css_to_tree(".root { font: 50px/2.0 Arial, sans-serif; }", single_node_tree);
    assert_eq!(styles[0].font_size, 50.0);
    assert_eq!(styles[0].line_height, Unit::Stretch(2.0));
    assert_eq!(*styles[0].font_family.as_ref().unwrap(), "Arial, sans-serif".into());
}

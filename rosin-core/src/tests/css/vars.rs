use super::util::*;

#[test]
fn vars_accept() {
    fn parse_color(s: &str) -> color::AlphaColor<Srgb> {
        color::parse_color(s).unwrap().to_alpha_color::<Srgb>()
    }

    fn build(angle: GradientAngle, space: color::ColorSpaceTag, hue: color::HueDirection, stops: &[(f32, color::AlphaColor<Srgb>)]) -> GradientStack {
        let mut g = LinearGradient::new(angle).with_interpolation_space(space).with_hue_direction(hue);
        for (pos, col) in stops {
            g = g.add_stop(*pos, *col);
        }
        GradientStackBuilder::new().add_linear(g).build()
    }

    // keyword
    let styles = apply_css_to_tree(".root { --dir: column; display: var(--dir); }", single_node_tree);
    assert_eq!(styles[0].display, Some(Direction::Column));

    // number/percent
    let styles = apply_css_to_tree(".root { --op: 10%; opacity: var(--op); }", single_node_tree);
    assert_eq!(styles[0].opacity, 0.1);

    // multi-token shorthand as a whole
    let styles = apply_css_to_tree(".root { --ol: 2px blue; outline: var(--ol); }", single_node_tree);
    assert_eq!(styles[0].outline_width, Length::Px(2.0));
    assert_eq!(styles[0].outline_color, color::palette::css::BLUE);

    // multi-token shorthand as parts
    let styles = apply_css_to_tree(".root { --w: 3px; --c: rgb(0, 0, 255); outline: var(--w) var(--c); }", single_node_tree);
    assert_eq!(styles[0].outline_width, Length::Px(3.0));
    assert_eq!(styles[0].outline_color, color::palette::css::BLUE);

    // 4-value property (token list)
    let styles = apply_css_to_tree(".root { --sp: 2s 3s 4s 5s; space: var(--sp); }", single_node_tree);
    assert_eq!(styles[0].top, Unit::Stretch(2.0));
    assert_eq!(styles[0].right, Unit::Stretch(3.0));
    assert_eq!(styles[0].bottom, Unit::Stretch(4.0));
    assert_eq!(styles[0].left, Unit::Stretch(5.0));

    // missing var with fallback
    let styles = apply_css_to_tree(".root { opacity: var(--missing, 10%); }", single_node_tree);
    assert_eq!(styles[0].opacity, 0.1);

    // nested fallback: prefer --op, else 20%
    let styles = apply_css_to_tree(".root { --op: 10%; opacity: var(--missing, var(--op, 20%)); }", single_node_tree);
    assert_eq!(styles[0].opacity, 0.1);

    // nested fallback when inner is also missing -> 20%
    let styles = apply_css_to_tree(".root { opacity: var(--missing, var(--also_missing, 20%)); }", single_node_tree);
    assert_eq!(styles[0].opacity, 0.2);

    // fallback that is a keyword (initial)
    let styles = apply_css_to_tree(".root { --reset: initial; display: var(--nope, var(--reset)); }", single_node_tree);
    assert_eq!(styles[0].display, Style::default().display);

    // parent defines, child consumes
    let styles = apply_css_to_tree(".parent { --dir: row; } .child { display: var(--dir); }", one_child_tree);
    assert_eq!(styles[0].display, Style::default().display);
    assert_eq!(styles[1].display, Some(Direction::Row));

    // child overrides parent var
    let styles = apply_css_to_tree(".parent { --dir: row; } .child { --dir: column-reverse; display: var(--dir); }", one_child_tree);
    assert_eq!(styles[1].display, Some(Direction::ColumnReverse));

    // siblings: override should not leak to other sibling
    let styles = apply_css_to_tree(".parent { --op: 10%; } .left { opacity: var(--op); } .right { --op: 50%; opacity: var(--op); }", two_child_tree);
    assert_eq!(styles[1].opacity, 0.1);
    assert_eq!(styles[2].opacity, 0.5);

    // vars can reference vars (aliasing)
    let styles = apply_css_to_tree(".root { --b: 10%; --a: var(--b); opacity: var(--a); }", single_node_tree);
    assert_eq!(styles[0].opacity, 0.1);

    // aliasing with parent override
    let styles = apply_css_to_tree(".parent { --b: 10%; --a: var(--b); } .child { --b: 20%; opacity: var(--a); }", one_child_tree);
    assert_eq!(styles[1].opacity, 0.2);

    // later custom property declaration wins
    let styles = apply_css_to_tree(".root { --op: 10%; --op: 20%; opacity: var(--op); }", single_node_tree);
    assert_eq!(styles[0].opacity, 0.2);

    // var is usable even if declared later in the same rule (resolution uses computed custom prop)
    let styles = apply_css_to_tree(".root { opacity: var(--op); --op: 10%; }", single_node_tree);
    assert_eq!(styles[0].opacity, 0.1);

    // `initial` via var
    let styles = apply_css_to_tree(".root { --p: fixed; position: var(--p); }", single_node_tree);
    assert_eq!(styles[0].position, Position::Fixed);

    let styles = apply_css_to_tree(".root { --p: initial; position: var(--p); }", single_node_tree);
    assert_eq!(styles[0].position, Style::default().position);

    // `inherit` via var (from parent)
    let styles = apply_css_to_tree(".parent { --p: fixed; position: var(--p); } .child { --kw: inherit; position: var(--kw); }", one_child_tree);
    assert_eq!(styles[0].position, Position::Fixed);
    assert_eq!(styles[1].position, Position::Fixed);

    let shadow_blue: Option<Arc<[TextShadow]>> = Some(Arc::new([TextShadow {
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

    // color token via var
    let styles = apply_css_to_tree(".root { --c: #00F; text-shadow: 5px 10px 15px var(--c); }", single_node_tree);
    assert_eq!(styles[0].text_shadow, shadow_blue);

    // currentcolor via var
    let styles = apply_css_to_tree(".root { --c: currentcolor; text-shadow: 5px 10px 15px var(--c); }", single_node_tree);
    assert_eq!(styles[0].text_shadow, shadow_current_color);

    // entire value via var (including fallback)
    let styles = apply_css_to_tree(".root { --ts: 5px 10px 15px blue; text-shadow: var(--ts); }", single_node_tree);
    assert_eq!(styles[0].text_shadow, shadow_blue);

    let styles = apply_css_to_tree(".root { text-shadow: var(--missing, none); }", single_node_tree);
    assert_eq!(styles[0].text_shadow, Style::default().text_shadow);

    let c_fff = parse_color("#fff");
    let c_000 = parse_color("#000");
    let c_red = parse_color("red");
    let c_blue = parse_color("blue");

    // colors via var() inside gradient
    let expected = build(GradientAngle::ToTop, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.0, c_fff), (1.0, c_000)]);
    let styles = apply_css_to_tree(".root { --a: #fff; --b: #000; background-image: linear-gradient(to top, var(--a), var(--b)); }", single_node_tree);
    assert_eq!(styles[0].background_image, Some(expected.clone()));

    // angle via var()
    let expected = build(GradientAngle::ToRight, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.0, c_fff), (1.0, c_000)]);
    let styles = apply_css_to_tree(".root { --ang: to right; background-image: linear-gradient(var(--ang), #fff, #000); }", single_node_tree);
    assert_eq!(styles[0].background_image, Some(expected));

    // interpolation clause via var() that expands to multiple tokens ("in oklab")
    let expected = build(GradientAngle::ToRight, color::ColorSpaceTag::Oklab, color::HueDirection::Shorter, &[(0.0, c_red), (1.0, c_blue)]);
    let styles = apply_css_to_tree(".root { --interp: in oklab; background-image: linear-gradient(to right var(--interp), red, blue); }", single_node_tree);
    assert_eq!(styles[0].background_image, Some(expected));

    // hue-direction via var() ("longer hue") in a hue-sensitive space
    let expected = build(
        GradientAngle::ToTop,
        color::ColorSpaceTag::Lch,
        color::HueDirection::Longer,
        &[(0.0, parse_color("hsl(10 100% 50%)")), (1.0, parse_color("hsl(350 100% 50%)"))],
    );
    let styles = apply_css_to_tree(
        ".root { --h: longer hue; background-image: linear-gradient(to top in lch var(--h), hsl(10 100% 50%), hsl(350 100% 50%)); }",
        single_node_tree,
    );
    assert_eq!(styles[0].background_image, Some(expected));

    // var() inside a color function used by gradient stops
    let expected = build(
        GradientAngle::ToBottom,
        color::ColorSpaceTag::Srgb,
        color::HueDirection::Shorter,
        &[(0.0, parse_color("rgb(255 255 255)")), (1.0, parse_color("rgb(0 0 0)"))],
    );
    let styles = apply_css_to_tree(
        ".root { --r: 255; --g: 255; --b: 255; background-image: linear-gradient(rgb(var(--r) var(--g) var(--b)), #000); }",
        single_node_tree,
    );
    assert_eq!(styles[0].background_image, Some(expected));

    // entire gradient function via var()
    let expected = build(GradientAngle::ToTop, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.0, c_fff), (1.0, c_000)]);
    let styles = apply_css_to_tree(".root { --bg: linear-gradient(to top, #fff, #000); background-image: var(--bg); }", single_node_tree);
    assert_eq!(styles[0].background_image, Some(expected));

    // multiple gradients via var() (expands to a comma-separated list)
    let expected = GradientStackBuilder::new()
        .add_linear(
            LinearGradient::new(GradientAngle::ToTop)
                .with_interpolation_space(color::ColorSpaceTag::Srgb)
                .with_hue_direction(color::HueDirection::Shorter)
                .add_stop(0.0_f32, c_fff)
                .add_stop(1.0_f32, c_000),
        )
        .add_linear(
            LinearGradient::new(GradientAngle::ToRight)
                .with_interpolation_space(color::ColorSpaceTag::Srgb)
                .with_hue_direction(color::HueDirection::Shorter)
                .add_stop(0.0_f32, c_red)
                .add_stop(1.0_f32, c_blue),
        )
        .build();
    let styles = apply_css_to_tree(
        ".root { --bg: linear-gradient(to top, #fff, #000), linear-gradient(to right, red, blue); background-image: var(--bg); }",
        single_node_tree,
    );
    assert_eq!(styles[0].background_image, Some(expected));

    // Inheritance of vars used inside background-image (parent -> child)
    let expected = build(GradientAngle::ToTop, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.0, c_fff), (1.0, c_000)]);
    let styles =
        apply_css_to_tree(".parent { --a: #fff; --b: #000; } .child { background-image: linear-gradient(to top, var(--a), var(--b)); }", one_child_tree);
    assert_eq!(styles[1].background_image, Some(expected));

    // child overrides just one stop
    let expected = build(GradientAngle::ToTop, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.0, c_red), (1.0, c_000)]);
    let styles = apply_css_to_tree(
        ".parent { --a: #fff; --b: #000; } .child { --a: red; background-image: linear-gradient(to top, var(--a), var(--b)); }",
        one_child_tree,
    );
    assert_eq!(styles[1].background_image, Some(expected));
}

#[test]
fn vars_reject() {
    fn reject(css_body: &str) {
        let css = format!(".root {{ --a: 10%; {} }}", css_body);
        let styles = apply_css_to_tree(&css, single_node_tree);

        assert_eq!(styles[0].opacity, Style::default().opacity, "expected parser to reject via var(): {}", css_body);
    }

    for body in [
        // var() syntax errors / tokenization issues
        "opacity: var(--a;",             // missing ')'
        "opacity: var(--a));",           // extra ')'
        "opacity: var(--a 10%);",        // missing comma between name and fallback
        "opacity: var(--a, var(--b);",   // inner var() missing ')'
        "opacity: var(--a, 10%) extra;", // trailing token after value
        "opacity: var(--a, 10%),;",      // unexpected comma after value
        "opacity: var(--a, 10%) / 2;",   // unexpected '/'
        // missing var() with no usable fallback
        "opacity: var(--missing);",                                   // missing custom property with no fallback
        "opacity: var(--missing, var(--also_missing));",              // fallback var() also missing with no fallback
        "opacity: var(--missing, var(--also_missing, var(--nope)));", // fallback resolves to missing var()
        // fallback exists but is wrong type for opacity
        "opacity: var(--missing, blue);",       // fallback is not a number/percentage
        "opacity: var(--missing, 10px);",       // fallback length is invalid for opacity
        "opacity: var(--missing, 10% 20%);",    // fallback has too many components
        "opacity: var(--missing, 10%, 20%);",   // fallback uses comma-separated list
        "opacity: var(--missing, rgb(0,0,0));", // fallback color is invalid for opacity
        "opacity: var(--missing, none);",       // fallback keyword invalid for opacity
        // custom prop exists but expands to invalid tokens for opacity
        "--op: 10 %; opacity: var(--op);",      // split percent token
        "--op: 10 . %; opacity: var(--op);",    // malformed numeric tokenization
        "--op: 10%%; opacity: var(--op);",      // malformed percent token
        "--op: ; opacity: var(--op);",          // empty custom property value
        "--op: 10% foo; opacity: var(--op);",   // trailing token after value
        "--op: 10% / 2; opacity: var(--op);",   // unexpected '/' in value
        "--op: var(--x,); opacity: var(--op);", // empty fallback in inner var()
        "--op: var(); opacity: var(--op);",     // var() missing name
        // cycles
        "--a: var(--a); opacity: var(--a);",                               // self-referential cycle
        "--a: var(--b); --b: var(--a); opacity: var(--a);",                // two-node cycle
        "--a: var(--b); --b: var(--c); --c: var(--a); opacity: var(--a);", // three-node cycle
    ] {
        reject(body);
    }
}

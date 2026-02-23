use super::util::*;

fn parse_color(s: &str) -> color::AlphaColor<Srgb> {
    color::parse_color(s).unwrap().to_alpha_color::<Srgb>()
}

fn stack_linear(angle: GradientAngle, space: color::ColorSpaceTag, hue: color::HueDirection, stops: &[(f32, color::AlphaColor<Srgb>)]) -> GradientStack {
    let mut g = LinearGradient::new(angle).with_interpolation_space(space).with_hue_direction(hue);

    for (pos, col) in stops {
        g = g.add_stop(*pos, *col);
    }

    GradientStackBuilder::new().add_linear(g).build()
}

fn stack_linear_str(angle: GradientAngle, space: color::ColorSpaceTag, hue: color::HueDirection, stops: &[(f32, &str)]) -> GradientStack {
    let stops = stops.iter().map(|(p, s)| (*p, parse_color(s))).collect::<Vec<_>>();
    stack_linear(angle, space, hue, &stops)
}

#[track_caller]
fn expect_single(css: &str, expected: Option<GradientStack>) {
    let styles = apply_css_to_tree(css, single_node_tree);
    assert_eq!(styles[0].background_image, expected);
}

#[track_caller]
fn expect_tree(tree: fn(&Stylesheet, &mut Ui<Stylesheet, ()>), css: &str, idx: usize, expected: Option<GradientStack>) {
    let styles = apply_css_to_tree(css, tree);
    assert_eq!(styles[idx].background_image, expected);
}

#[test]
fn background_image_keywords_and_inheritance() {
    let c_fff = parse_color("#fff");
    let c_000 = parse_color("#000");

    let base = stack_linear(GradientAngle::ToTop, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.0, c_fff), (1.0, c_000)]);

    // none
    expect_single(".root { background-image: none; }", Style::default().background_image);

    // normal use
    expect_single(".root { background-image: linear-gradient(to top, rgb(255, 255, 255), rgba(0, 0, 0, 1)); }", Some(base.clone()));

    // initial
    expect_tree(
        two_child_tree,
        ".child { background-image: linear-gradient(to top, #FFF, #000); } \
         .right { background-image: initial; }",
        1,
        Some(base.clone()),
    );
    expect_tree(
        two_child_tree,
        ".child { background-image: linear-gradient(to top, #FFF, #000); } \
         .right { background-image: initial; }",
        2,
        Style::default().background_image,
    );

    // inherit
    expect_tree(
        one_child_tree,
        ".parent { background-image: linear-gradient(to top, #FFF, #000); } \
         .child { background-image: inherit; }",
        0,
        Some(base.clone()),
    );
    expect_tree(
        one_child_tree,
        ".parent { background-image: linear-gradient(to top, #FFF, #000); } \
         .child { background-image: inherit; }",
        1,
        Some(base.clone()),
    );

    // uninherited default
    expect_tree(one_child_tree, ".parent { background-image: linear-gradient(to top, #FFF, #000); }", 0, Some(base.clone()));
    expect_tree(one_child_tree, ".parent { background-image: linear-gradient(to top, #FFF, #000); }", 1, Style::default().background_image);
}

#[test]
fn background_image_linear_gradient_parsing() {
    let c_fff = parse_color("#fff");
    let c_000 = parse_color("#000");
    let c_white = parse_color("white");
    let c_black = parse_color("black");
    let c_red = parse_color("red");
    let c_blue = parse_color("blue");
    let c_transparent = parse_color("transparent");

    // directions
    for (css, angle) in [
        (".root { background-image: linear-gradient(#fff, #000); }", GradientAngle::ToBottom),
        (".root { background-image: linear-gradient(to right, #fff, #000); }", GradientAngle::ToRight),
        (".root { background-image: linear-gradient(to left, #fff, #000); }", GradientAngle::ToLeft),
        (".root { background-image: linear-gradient(to top right, #fff, #000); }", GradientAngle::ToTopRight),
        (".root { background-image: linear-gradient(to top left, #fff, #000); }", GradientAngle::ToTopLeft),
        (".root { background-image: linear-gradient(to bottom right, #fff, #000); }", GradientAngle::ToBottomRight),
        (".root { background-image: linear-gradient(to bottom left, #fff, #000); }", GradientAngle::ToBottomLeft),
    ] {
        expect_single(css, Some(stack_linear(angle, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.0, c_fff), (1.0, c_000)])));
    }

    // named colors
    expect_single(
        ".root { background-image: linear-gradient(to bottom, white, black); }",
        Some(stack_linear(GradientAngle::ToBottom, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.0, c_white), (1.0, c_black)])),
    );

    // degrees / radians
    for (css, angle) in [
        (".root { background-image: linear-gradient(0deg, #fff, #000); }", GradientAngle::Degrees(0.0)),
        (".root { background-image: linear-gradient(90deg, #fff, #000); }", GradientAngle::Degrees(90.0)),
        (".root { background-image: linear-gradient(-45deg, #fff, #000); }", GradientAngle::Degrees(-45.0)),
        (".root { background-image: linear-gradient(450deg, #fff, #000); }", GradientAngle::Degrees(450.0)),
        (".root { background-image: linear-gradient(3.1415927rad, #fff, #000); }", GradientAngle::Radians(std::f32::consts::PI)),
    ] {
        expect_single(css, Some(stack_linear(angle, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.0, c_fff), (1.0, c_000)])));
    }

    // explicit % stops baseline
    expect_single(
        ".root { background-image: linear-gradient(to top, #FFF 0%, #000 100%); }",
        Some(stack_linear(GradientAngle::ToTop, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.0, c_fff), (1.0, c_000)])),
    );

    // mid % stops
    expect_single(
        ".root { background-image: linear-gradient(to top, #FFF 25%, #000 75%); }",
        Some(stack_linear(GradientAngle::ToTop, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.25, c_fff), (0.75, c_000)])),
    );

    // decimal % stops
    expect_single(
        ".root { background-image: linear-gradient(to top, #FFF 12.5%, #000 87.5%); }",
        Some(stack_linear(GradientAngle::ToTop, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.125, c_fff), (0.875, c_000)])),
    );

    // hard stops
    expect_single(
        ".root { background-image: linear-gradient(to right, #fff 0% 50%, #000 50% 100%); }",
        Some(stack_linear(
            GradientAngle::ToRight,
            color::ColorSpaceTag::Srgb,
            color::HueDirection::Shorter,
            &[(0.0, c_fff), (0.5, c_fff), (0.5, c_000), (1.0, c_000)],
        )),
    );

    // parse_color syntax coverage
    for (css, angle, stops) in [
        (
            ".root { background-image: linear-gradient(to top, rgb(255, 255, 255), rgb(0, 0, 0)); }",
            GradientAngle::ToTop,
            &[(0.0_f32, "rgb(255, 255, 255)"), (1.0, "rgb(0, 0, 0)")],
        ),
        (
            ".root { background-image: linear-gradient(to top, rgb(255 255 255), rgb(0 0 0)); }",
            GradientAngle::ToTop,
            &[(0.0, "rgb(255 255 255)"), (1.0, "rgb(0 0 0)")],
        ),
        (
            ".root { background-image: linear-gradient(to top, rgb(255 255 255 / 100%), rgb(0 0 0 / 1)); }",
            GradientAngle::ToTop,
            &[(0.0, "rgb(255 255 255 / 100%)"), (1.0, "rgb(0 0 0 / 1)")],
        ),
        (
            ".root { background-image: linear-gradient(to right, hsl(0 100% 50%), hsla(240, 100%, 50%, 1)); }",
            GradientAngle::ToRight,
            &[(0.0, "hsl(0 100% 50%)"), (1.0, "hsla(240, 100%, 50%, 1)")],
        ),
        (
            ".root { background-image: linear-gradient(to right, hwb(0 0% 0%), hwb(120 0% 0% / 1)); }",
            GradientAngle::ToRight,
            &[(0.0, "hwb(0 0% 0%)"), (1.0, "hwb(120 0% 0% / 1)")],
        ),
        (
            ".root { background-image: linear-gradient(to bottom, lab(60% 40 50), lab(20% 0 0 / 1)); }",
            GradientAngle::ToBottom,
            &[(0.0, "lab(60% 40 50)"), (1.0, "lab(20% 0 0 / 1)")],
        ),
        (
            ".root { background-image: linear-gradient(to bottom, lch(60% 50 40), lch(20% 0 0 / 1)); }",
            GradientAngle::ToBottom,
            &[(0.0, "lch(60% 50 40)"), (1.0, "lch(20% 0 0 / 1)")],
        ),
        (
            ".root { background-image: linear-gradient(to left, oklab(0.7 0.1 0.05), oklab(0.2 0 0 / 1)); }",
            GradientAngle::ToLeft,
            &[(0.0, "oklab(0.7 0.1 0.05)"), (1.0, "oklab(0.2 0 0 / 1)")],
        ),
        (
            ".root { background-image: linear-gradient(to left, oklch(0.7 0.15 40), oklch(0.2 0 0 / 1)); }",
            GradientAngle::ToLeft,
            &[(0.0, "oklch(0.7 0.15 40)"), (1.0, "oklch(0.2 0 0 / 1)")],
        ),
        (
            ".root { background-image: linear-gradient(to right, color(display-p3 1 0 0), color(display-p3 0 0 1 / 1)); }",
            GradientAngle::ToRight,
            &[(0.0, "color(display-p3 1 0 0)"), (1.0, "color(display-p3 0 0 1 / 1)")],
        ),
        (
            ".root { background-image: linear-gradient(to right, color(rec2020 0 1 0), color(rec2020 1 0 0 / 1)); }",
            GradientAngle::ToRight,
            &[(0.0, "color(rec2020 0 1 0)"), (1.0, "color(rec2020 1 0 0 / 1)")],
        ),
    ] {
        expect_single(css, Some(stack_linear_str(angle, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, stops)));
    }

    // Color-space parsing (hue fixed to Shorter)
    for (css, space, angle) in [
        (".root { background-image: linear-gradient(to top in srgb, #fff, #000); }", color::ColorSpaceTag::Srgb, GradientAngle::ToTop),
        (".root { background-image: linear-gradient(to top in linear-srgb, #fff, #000); }", color::ColorSpaceTag::LinearSrgb, GradientAngle::ToTop),
        (".root { background-image: linear-gradient(to top in lab, #fff, #000); }", color::ColorSpaceTag::Lab, GradientAngle::ToTop),
        (".root { background-image: linear-gradient(to bottom in oklab, #fff, #000); }", color::ColorSpaceTag::Oklab, GradientAngle::ToBottom),
        (
            ".root { background-image: linear-gradient(to bottom in display-p3, #fff, #000); }",
            color::ColorSpaceTag::DisplayP3,
            GradientAngle::ToBottom,
        ),
        (".root { background-image: linear-gradient(to bottom in a98-rgb, #fff, #000); }", color::ColorSpaceTag::A98Rgb, GradientAngle::ToBottom),
        (
            ".root { background-image: linear-gradient(to bottom in prophoto-rgb, #fff, #000); }",
            color::ColorSpaceTag::ProphotoRgb,
            GradientAngle::ToBottom,
        ),
        (".root { background-image: linear-gradient(to bottom in rec2020, #fff, #000); }", color::ColorSpaceTag::Rec2020, GradientAngle::ToBottom),
        (
            ".root { background-image: linear-gradient(to bottom in aces2065-1, #fff, #000); }",
            color::ColorSpaceTag::Aces2065_1,
            GradientAngle::ToBottom,
        ),
        (".root { background-image: linear-gradient(to bottom in aces-cg, #fff, #000); }", color::ColorSpaceTag::AcesCg, GradientAngle::ToBottom),
        (".root { background-image: linear-gradient(to bottom in xyz-d50, #fff, #000); }", color::ColorSpaceTag::XyzD50, GradientAngle::ToBottom),
        (".root { background-image: linear-gradient(to bottom in xyz-d65, #fff, #000); }", color::ColorSpaceTag::XyzD65, GradientAngle::ToBottom),
    ] {
        expect_single(css, Some(stack_linear(angle, space, color::HueDirection::Shorter, &[(0.0, c_fff), (1.0, c_000)])));
    }

    // hue-direction parsing cases that actually use hue-sensitive colors
    for (css, space, hue, angle) in [
        (
            ".root { background-image: linear-gradient(to top in lch longer hue, hsl(10 100% 50%), hsl(350 100% 50%)); }",
            color::ColorSpaceTag::Lch,
            color::HueDirection::Longer,
            GradientAngle::ToTop,
        ),
        (
            ".root { background-image: linear-gradient(to top in lch increasing hue, hsl(10 100% 50%), hsl(350 100% 50%)); }",
            color::ColorSpaceTag::Lch,
            color::HueDirection::Increasing,
            GradientAngle::ToTop,
        ),
        (
            ".root { background-image: linear-gradient(to top in lch decreasing hue, hsl(10 100% 50%), hsl(350 100% 50%)); }",
            color::ColorSpaceTag::Lch,
            color::HueDirection::Decreasing,
            GradientAngle::ToTop,
        ),
        (
            ".root { background-image: linear-gradient(to right in hsl longer hue, hsl(10 100% 50%), hsl(350 100% 50%)); }",
            color::ColorSpaceTag::Hsl,
            color::HueDirection::Longer,
            GradientAngle::ToRight,
        ),
        (
            ".root { background-image: linear-gradient(to right in hwb shorter hue, hwb(10 0% 0%), hwb(350 0% 0%)); }",
            color::ColorSpaceTag::Hwb,
            color::HueDirection::Shorter,
            GradientAngle::ToRight,
        ),
        (
            ".root { background-image: linear-gradient(to bottom in oklch longer hue, hsl(10 100% 50%), hsl(350 100% 50%)); }",
            color::ColorSpaceTag::Oklch,
            color::HueDirection::Longer,
            GradientAngle::ToBottom,
        ),
    ] {
        expect_single(css, Some(stack_linear_str(angle, space, hue, &[(0.0, "hsl(10 100% 50%)"), (1.0, "hsl(350 100% 50%)")])));
    }

    // no angle, interpolation first
    expect_single(
        ".root { background-image: linear-gradient(in oklab, red, blue); }",
        Some(stack_linear(GradientAngle::ToBottom, color::ColorSpaceTag::Oklab, color::HueDirection::Shorter, &[(0.0, c_red), (1.0, c_blue)])),
    );

    // case/whitespace noise
    expect_single(
        ".root { background-image: LiNeAr-GrAdIeNt(  to  right   in   lab  ,  #fff  ,  #000  ); }",
        Some(stack_linear(GradientAngle::ToRight, color::ColorSpaceTag::Lab, color::HueDirection::Shorter, &[(0.0, c_fff), (1.0, c_000)])),
    );

    // transparent keyword
    expect_single(
        ".root { background-image: linear-gradient(transparent 0%, #000 100%); }",
        Some(stack_linear(GradientAngle::ToBottom, color::ColorSpaceTag::Srgb, color::HueDirection::Shorter, &[(0.0, c_transparent), (1.0, c_000)])),
    );
}

#[test]
fn background_image_multiple_gradients() {
    let c_fff = parse_color("#fff");
    let c_000 = parse_color("#000");
    let c_red = parse_color("red");
    let c_blue = parse_color("blue");

    // multiple gradients
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
                .with_interpolation_space(color::ColorSpaceTag::Oklab)
                .with_hue_direction(color::HueDirection::Shorter)
                .add_stop(0.0_f32, c_red)
                .add_stop(1.0_f32, c_blue),
        )
        .build();

    let styles =
        apply_css_to_tree(".root { background-image: linear-gradient(to top, #fff, #000), linear-gradient(to right in oklab, red, blue); }", single_node_tree);
    assert_eq!(styles[0].background_image, Some(expected));

    // multiple gradients with % stops
    let expected = GradientStackBuilder::new()
        .add_linear(
            LinearGradient::new(GradientAngle::Degrees(180.0))
                .with_interpolation_space(color::ColorSpaceTag::Srgb)
                .with_hue_direction(color::HueDirection::Shorter)
                .add_stop(0.0_f32, parse_color("rgb(255 255 255)"))
                .add_stop(1.0_f32, parse_color("rgb(0 0 0)")),
        )
        .add_linear(
            LinearGradient::new(GradientAngle::ToBottomLeft)
                .with_interpolation_space(color::ColorSpaceTag::Lch)
                .with_hue_direction(color::HueDirection::Increasing)
                .add_stop(0.0_f32, parse_color("hsl(10 100% 50%)"))
                .add_stop(0.5_f32, parse_color("hsl(180 100% 50%)"))
                .add_stop(1.0_f32, parse_color("hsl(350 100% 50%)")),
        )
        .build();

    let styles = apply_css_to_tree(
        ".root { background-image: \
            linear-gradient(180deg, rgb(255 255 255) 0%, rgb(0 0 0) 100%), \
            linear-gradient(to bottom left in lch increasing hue, \
                hsl(10 100% 50%) 0%, hsl(180 100% 50%) 50%, hsl(350 100% 50%) 100%); \
        }",
        single_node_tree,
    );
    assert_eq!(styles[0].background_image, Some(expected));
}

#[test]
fn background_image_reject() {
    fn reject(value: &str) {
        let css = format!(".root {{ background-image: {}; }}", value);
        let styles = apply_css_to_tree(&css, single_node_tree);
        assert_eq!(styles[0].background_image, Style::default().background_image, "expected parser to reject background-image: {}", value);
    }

    // Each of these should make `parse_background_image` return Err, causing the declaration to be rejected.
    for value in [
        "#fff",                                                      // not a function
        "foo",                                                       // not a function
        "radial-gradient(#fff, #000)",                               // unsupported function
        "url(test.png)",                                             // unsupported function
        "linear-gradient(, #fff, #000)",                             // leading comma / missing first argument
        "linear-gradient(,)",                                        // only commas / missing arguments
        "linear-gradient((#fff, #000)",                              // unexpected opening parenthesis
        "linear-gradient()",                                         // empty argument list
        "linear-gradient(#12, #000)",                                // invalid hex color length
        "linear-gradient(#fff -10px, #000)",                         // negative length used for stop position
        "linear-gradient(#fff #000)",                                // missing comma between color stops
        "linear-gradient(#fff %10, #000)",                           // malformed percentage token order
        "linear-gradient(#fff 10 % , #000)",                         // whitespace splits percentage token
        "linear-gradient(#fff 10, #000)",                            // unitless number used for stop position
        "linear-gradient(#fff 10.% , #000)",                         // malformed percentage token (dot + space)
        "linear-gradient(#fff 10% #000 20%)",                        // missing comma between stops
        "linear-gradient(#fff 10% 20% 30%, #000)",                   // too many positions on a color stop
        "linear-gradient(#fff 10% 20px, #000)",                      // mixed units in a color stop position
        "linear-gradient(#fff 10%, , #000 20%)",                     // empty stop between commas
        "linear-gradient(#fff 10%, #000 / 50%)",                     // unexpected `/` token in stop list
        "linear-gradient(#fff 10%, #000 20)",                        // unitless number used for stop position
        "linear-gradient(#fff 10%, #000 20% ,)",                     // trailing comma after last stop
        "linear-gradient(#fff 10%, #000 20% / 30%)",                 // unexpected `/` token in stop list
        "linear-gradient(#fff 10%, #000 20% #111)",                  // missing comma before third stop
        "linear-gradient(#fff 10%, #000 20% 30% 40%)",               // too many positions on last stop
        "linear-gradient(#fff 10%, #000 20% 30%, #111 40% 50% 60%)", // too many positions on a stop
        "linear-gradient(#fff 10%, #000 20%%)",                      // malformed percentage token
        "linear-gradient(#fff 10%, #000 20px)",                      // length used for stop position
        "linear-gradient(#fff 10%, #000,, #111)",                    // double comma between stops
        "linear-gradient(#fff 10%,#000 20% #111 30%)",               // missing comma between second and third stops
        "linear-gradient(#fff 10deg, #000)",                         // angle used for stop position
        "linear-gradient(#fff 10px, #000)",                          // length used for stop position (percent only)
        "linear-gradient(#fff, , #000)",                             // empty argument between commas
        "linear-gradient(#fff, , 50%, #000)",                        // empty argument before hint
        "linear-gradient(#fff, #000 foo)",                           // trailing garbage after color stop
        "linear-gradient(#fff, #000,)",                              // trailing comma inside function
        "linear-gradient(#fff, #000",                                // missing closing parenthesis
        "linear-gradient(#fff, #000) ,,linear-gradient(#fff, #000)", // multiple commas
        "linear-gradient(#fff, #000) !important",                    // unexpected !important token in value
        "linear-gradient(#fff, #000) extra",                         // trailing tokens after function
        "linear-gradient(#fff, #000),",                              // extra trailing token after function
        "linear-gradient(#fff, #000))",                              // extra closing parenthesis
        "linear-gradient(#fff, 20%, 30%, #000)",                     // bare stop positions without colors
        "linear-gradient(#fff, 50, #000)",                           // color hint uses unitless number
        "linear-gradient(#fff, 50% 60%, #000)",                      // color hint has two positions
        "linear-gradient(#fff, 50%, , #000)",                        // empty argument after hint
        "linear-gradient(#fff, 50px, #000)",                         // color hint uses length (percent only)
        "linear-gradient(#fff,, #000)",                              // double comma between stops
        "linear-gradient(#fff,)",                                    // dangling comma / missing final color stop
        "linear-gradient(#fff; #000)",                               // wrong separator (`;` not `,`)
        "linear-gradient(#fff)",                                     // needs at least 2 color stops
        "linear-gradient(#ggg, #000)",                               // invalid hex color digits
        "linear-gradient(10%, #fff, #000)",                          // percentage used where direction/angle expected
        "linear-gradient(10px, #fff, #000)",                         // length used where direction/angle expected
        "linear-gradient(color(srgb 1 1 1), color(srgb 0 0))",       // invalid `color()` function (missing component)
        "linear-gradient(hsl(0 100% 50%), hsl(0 0%))",               // invalid hsl() arguments
        "linear-gradient(in 1, #fff, #000)",                         // invalid interpolation method token
        "linear-gradient(in nope, #fff, #000)",                      // unknown interpolation method
        "linear-gradient(in oklab red, blue)",                       // missing comma after interpolation method
        "linear-gradient(in oklab, red blue)",                       // missing comma between color stops
        "linear-gradient(in oklab, red, blue foo)",                  // trailing garbage after last stop
        "linear-gradient(in oklab, red, blue,)",                     // trailing comma after last stop
        "linear-gradient(in oklab, red)",                            // only one color stop after interpolation method
        "linear-gradient(in oklab,, red, blue)",                     // double comma after interpolation method
        "linear-gradient(in oklab,)",                                // no color stops after interpolation method
        "linear-gradient(in, #fff, #000)",                           // missing interpolation method after `in`
        "linear-gradient(left, #fff, #000)",                         // missing `to` keyword for side/corner
        "linear-gradient(oklab(0.5 0 0), oklab())",                  // invalid color function arguments
        "linear-gradient(rgb(255 255 255 / 1), rgb())",              // invalid color function arguments
        "linear-gradient(rgb(255, 255, 255) rgb(0, 0, 0))",          // missing comma between color stops
        "linear-gradient(rgb(255, 255, 255), rgb(0, 0, 0) foo)",     // trailing garbage after color stop
        "linear-gradient(to left left, #fff, #000)",                 // duplicate direction keyword
        "linear-gradient(to left right top, #fff, #000)",            // too many direction keywords
        "linear-gradient(to left right, #fff, #000)",                // conflicting direction keywords
        "linear-gradient(to left top bottom, #fff, #000)",           // contradictory directions
        "linear-gradient(to left-bottom, #fff, #000)",               // invalid combined direction token
        "linear-gradient(to left, #fff)",                            // only one color stop after direction
        "linear-gradient(to left,)",                                 // direction present but no color stops
        "linear-gradient(to north, #fff, #000)",                     // invalid direction keyword
        "linear-gradient(to top #fff, #000)",                        // missing comma after direction
        "linear-gradient(to top bottom, #fff, #000)",                // conflicting direction keywords
        "linear-gradient(to up, #fff, #000)",                        // invalid direction keyword
        "linear-gradient(to, #fff, #000)",                           // missing direction after `to`
        "linear-gradient[#fff, #000]",                               // wrong brackets instead of parentheses
    ] {
        reject(value);
    }
}

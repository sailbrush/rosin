use super::util::*;

#[test]
fn css_color_parsing() {
    // ---------- NAMED COLORS ----------

    // Transparent
    let styles = apply_css_to_tree(".root { background-color: transparent; }", single_node_tree);
    assert_eq!(styles[0].background_color, color::palette::css::TRANSPARENT);

    // Lowercase named color
    let styles = apply_css_to_tree(".root { background-color: cyan; }", single_node_tree);
    assert_eq!(styles[0].background_color, color::palette::css::CYAN);

    // Uppercase named color
    let styles = apply_css_to_tree(".root { background-color: PURPLE; }", single_node_tree);
    assert_eq!(styles[0].background_color, color::palette::css::PURPLE);

    // ---------- HEX ----------

    // Lowercase hex
    let styles = apply_css_to_tree(".root { background-color: #6a5acd; }", single_node_tree);
    assert_eq!(styles[0].background_color, color::parse_color("#6a5acd").unwrap().to_alpha_color::<Srgb>());

    // Uppercase hex
    let styles = apply_css_to_tree(".root { background-color: #FFDEAD; }", single_node_tree);
    assert_eq!(styles[0].background_color, color::parse_color("#ffdead").unwrap().to_alpha_color::<Srgb>());

    // Mixed case hex
    let styles = apply_css_to_tree(".root { background-color: #F5dEb3; }", single_node_tree);
    assert_eq!(styles[0].background_color, color::parse_color("#f5deb3").unwrap().to_alpha_color::<Srgb>());

    // Hex with alpha
    let styles = apply_css_to_tree(".root { background-color: #f5fffa5a; }", single_node_tree);
    assert_eq!(styles[0].background_color, color::parse_color("#f5fffa5a").unwrap().to_alpha_color::<Srgb>());

    // Abbreviated hex
    let styles = apply_css_to_tree(".root { background-color: #3c9; }", single_node_tree);
    assert_eq!(styles[0].background_color, color::parse_color("#3c9").unwrap().to_alpha_color::<Srgb>());

    // Abbreviated hex with alpha
    let styles = apply_css_to_tree(".root { background-color: #3c99; }", single_node_tree);
    assert_eq!(styles[0].background_color, color::parse_color("#3c99").unwrap().to_alpha_color::<Srgb>());

    // ---------- RGB ----------

    // Legacy lowercase rgb()
    let styles = apply_css_to_tree(".root { background-color: rgb(76, 187, 23); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgb8(76, 187, 23));

    // Legacy lowercase rgb() with percentages
    let styles = apply_css_to_tree(".root { background-color: rgb(76%, 87%, 23%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("rgb(76%, 87%, 23%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Legacy uppercase rgb() with percentages
    let styles = apply_css_to_tree(".root { background-color: RGB(76%, 87%, 23%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("RGB(76%, 87%, 23%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase rgb() with percentages
    let styles = apply_css_to_tree(".root { background-color: rgb(32% 78% 70%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("rgb(32% 78% 70%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase rgb() with mixed percentages
    let styles = apply_css_to_tree(".root { background-color: rgb(32 50% 50%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("rgb(32 50% 50%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase rgb() with alpha percent
    let styles = apply_css_to_tree(".root { background-color: rgb(32 178 170 / 20%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("rgb(32 178 170 / 20%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase rgb() with percentages
    let styles = apply_css_to_tree(".root { background-color: RGB(32% 78% 70%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("RGB(32% 78% 70%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase rgb() with mixed percentages
    let styles = apply_css_to_tree(".root { background-color: RGB(32 50% 50%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("RGB(32 50% 50%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase rgb() with alpha percent
    let styles = apply_css_to_tree(".root { background-color: RGB(32 178 170 / 20%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("RGB(32 178 170 / 20%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Legacy uppercase RGB()
    let styles = apply_css_to_tree(".root { background-color: RGB(76, 187, 23); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgb8(76, 187, 23));

    // Lowercase rgb()
    let styles = apply_css_to_tree(".root { background-color: rgb(32 178 170); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgb8(32, 178, 170));

    // Lowercase rgb() with alpha
    let styles = apply_css_to_tree(".root { background-color: rgb(32 178 170 / 0.2); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(32, 178, 170, 51));

    // Uppercase rgb()
    let styles = apply_css_to_tree(".root { background-color: RGB(32 178 170); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgb8(32, 178, 170));

    // Uppercase rgb() with float alpha
    let styles = apply_css_to_tree(".root { background-color: RGB(32 178 170 / 0.2); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(32, 178, 170, 51));

    // ---------- RGBA ----------

    // Lowercase rgba()
    let styles = apply_css_to_tree(".root { background-color: rgba(50, 205, 51); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(50, 205, 51, 255));

    // Lowercase rgba() with alpha
    let styles = apply_css_to_tree(".root { background-color: rgba(50, 205, 51, 0.3); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(50, 205, 51, 77));

    // Lowercase rgba() with percent alpha
    let styles = apply_css_to_tree(".root { background-color: rgba(50, 205, 51, 30%); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(50, 205, 51, 77));

    // Lowercase rgba() with space-separated values
    let styles = apply_css_to_tree(".root { background-color: rgba(50 205 51); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(50, 205, 51, 255));

    // Lowercase rgba() with space-separated values and alpha
    let styles = apply_css_to_tree(".root { background-color: rgba(50 205 51 / 0.3); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(50, 205, 51, 77));

    // Lowercase rgba() with space-separated values and percent alpha
    let styles = apply_css_to_tree(".root { background-color: rgba(50 205 51 / 30%); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(50, 205, 51, 77));

    // Uppercase RGBA()
    let styles = apply_css_to_tree(".root { background-color: RGBA(50, 205, 51); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(50, 205, 51, 255));

    // Uppercase RGBA() with alpha
    let styles = apply_css_to_tree(".root { background-color: RGBA(50, 205, 51, 0.3); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(50, 205, 51, 77));

    // Uppercase RGBA() with percent alpha
    let styles = apply_css_to_tree(".root { background-color: RGBA(50, 205, 51, 30%); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(50, 205, 51, 77));

    // Uppercase RGBA() with space-separated values
    let styles = apply_css_to_tree(".root { background-color: RGBA(50 205 51); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(50, 205, 51, 255));

    // Uppercase RGBA() with space-separated values and alpha
    let styles = apply_css_to_tree(".root { background-color: RGBA(50 205 51 / 0.3); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(50, 205, 51, 77));

    // Uppercase RGBA() with space-separated values and percent alpha
    let styles = apply_css_to_tree(".root { background-color: RGBA(50 205 51 / 30%); }", single_node_tree);
    assert_eq!(styles[0].background_color, Color::from_rgba8(50, 205, 51, 77));

    // ---------- HSL ----------

    // Lowercase hsl()
    let styles = apply_css_to_tree(".root { background-color: hsl(50 60% 40%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("hsl(50 60% 40%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase hsl() with alpha
    let styles = apply_css_to_tree(".root { background-color: hsl(50, 60%, 40%, 0.3); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("hsl(50, 60%, 40%, 0.3)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase hsl() with percent alpha
    let styles = apply_css_to_tree(".root { background-color: hsl(50, 60%, 40%, 30%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("hsl(50, 60%, 40%, 30%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase hsl() with space-separated values
    let styles = apply_css_to_tree(".root { background-color: hsl(50 60% 40%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("hsl(50 60% 40%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase hsl() with space-separated values and alpha
    let styles = apply_css_to_tree(".root { background-color: hsl(50 60% 40% / 0.3); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("hsl(50 60% 40% / 0.3)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase hsl() with space-separated values and percent alpha
    let styles = apply_css_to_tree(".root { background-color: hsl(50 60% 40% / 30%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("hsl(50 60% 40% / 30%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase HSL()
    let styles = apply_css_to_tree(".root { background-color: HSL(50, 60%, 40%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("HSL(50, 60%, 40%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase HSL() with alpha
    let styles = apply_css_to_tree(".root { background-color: HSL(50, 60%, 40%, 0.3); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("HSL(50, 60%, 40%, 0.3)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase HSL() with percent alpha
    let styles = apply_css_to_tree(".root { background-color: HSL(50, 60%, 40%, 30%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("HSL(50, 60%, 40%, 30%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase HSL() with space-separated values
    let styles = apply_css_to_tree(".root { background-color: HSL(50 60% 40%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("HSL(50 60% 40%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase HSL() with space-separated values and alpha
    let styles = apply_css_to_tree(".root { background-color: HSL(50 60% 40% / 0.3); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("HSL(50 60% 40% / 0.3)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase HSL() with space-separated values and percent alpha
    let styles = apply_css_to_tree(".root { background-color: HSL(50 60% 40% / 30%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("HSL(50 60% 40% / 30%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // ---------- HWB ----------

    // Lowercase hwb()
    let styles = apply_css_to_tree(".root { background-color: hwb(240 7% 13%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("hwb(240 7% 13%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase hwb() with alpha
    let styles = apply_css_to_tree(".root { background-color: hwb(12 50% 0% / 0.3); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("hwb(12 50% 0% / 0.3)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase HWB()
    let styles = apply_css_to_tree(".root { background-color: HWB(12 50% 0%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("HWB(12 50% 0%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase HWB() with alpha
    let styles = apply_css_to_tree(".root { background-color: HWB(12 50% 0% / 0.3); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("HWB(12 50% 0% / 0.3)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // ---------- LAB ----------

    // Lowercase lab()
    let styles = apply_css_to_tree(".root { background-color: lab(29.2345% 39.3825 20.0664); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("lab(29.2345% 39.3825 20.0664)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase lab() with alpha
    let styles = apply_css_to_tree(".root { background-color: lab(29.2345% 39.3825 20.0664 / 50%); }", single_node_tree);
    assert_eq!(
        styles[0].background_color.to_rgba8(),
        color::parse_color("lab(29.2345% 39.3825 20.0664 / 50%)")
            .unwrap()
            .to_alpha_color::<Srgb>()
            .to_rgba8()
    );

    // Uppercase LAB()
    let styles = apply_css_to_tree(".root { background-color: LAB(50% -50 50); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("LAB(50% -50 50)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase LAB() with alpha
    let styles = apply_css_to_tree(".root { background-color: LAB(50% -50 50 / 0.3); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("LAB(50% -50 50 / 0.3)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // ---------- LCH ----------

    // Lowercase lch()
    let styles = apply_css_to_tree(".root { background-color: lch(48.56% 91.1 298.05); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("lch(48.56% 91.1 298.05)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase lch() with alpha
    let styles = apply_css_to_tree(".root { background-color: lch(48.56% 91.1 298.05 / 50%); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("lch(48.56% 91.1 298.05 / 50%)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase LCH()
    let styles = apply_css_to_tree(".root { background-color: LCH(70% 40 200); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("LCH(70% 40 200)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase LCH() with alpha
    let styles = apply_css_to_tree(".root { background-color: LCH(70% 40 200 / 0.3); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("LCH(70% 40 200 / 0.3)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // ---------- OKLAB ----------

    // Lowercase oklab()
    let styles = apply_css_to_tree(".root { background-color: oklab(50% 0 0); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("oklab(50% 0 0)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase oklab() with alpha
    let styles = apply_css_to_tree(".root { background-color: oklab(40.1% 0.1143 0.045 / 50%); }", single_node_tree);
    assert_eq!(
        styles[0].background_color.to_rgba8(),
        color::parse_color("oklab(40.1% 0.1143 0.045 / 50%)")
            .unwrap()
            .to_alpha_color::<Srgb>()
            .to_rgba8()
    );

    // Uppercase OKLAB()
    let styles = apply_css_to_tree(".root { background-color: OKLAB(40.1% 0.1143 0.045); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("OKLAB(40.1% 0.1143 0.045)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase OKLAB() with alpha
    let styles = apply_css_to_tree(".root { background-color: OKLAB(40.1% 0.1143 0.045 / 0.7); }", single_node_tree);
    assert_eq!(
        styles[0].background_color.to_rgba8(),
        color::parse_color("OKLAB(40.1% 0.1143 0.045 / 0.7)")
            .unwrap()
            .to_alpha_color::<Srgb>()
            .to_rgba8()
    );

    // ---------- OKLCH ----------

    // Lowercase oklch()
    let styles = apply_css_to_tree(".root { background-color: oklch(45.9% 0.167 350.36); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("oklch(45.9% 0.167 350.36)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Lowercase oklch() with alpha
    let styles = apply_css_to_tree(".root { background-color: oklch(44.14% 0.106 277.82 / 50%); }", single_node_tree);
    assert_eq!(
        styles[0].background_color.to_rgba8(),
        color::parse_color("oklch(44.14% 0.106 277.82 / 50%)")
            .unwrap()
            .to_alpha_color::<Srgb>()
            .to_rgba8()
    );

    // Uppercase OKLCH()
    let styles = apply_css_to_tree(".root { background-color: OKLCH(44.14% 0.106 277.82); }", single_node_tree);
    assert_eq!(styles[0].background_color.to_rgba8(), color::parse_color("OKLCH(44.14% 0.106 277.82)").unwrap().to_alpha_color::<Srgb>().to_rgba8());

    // Uppercase OKLCH() with alpha
    let styles = apply_css_to_tree(".root { background-color: OKLCH(45.9% 0.167 350.36 / 0.7); }", single_node_tree);
    assert_eq!(
        styles[0].background_color.to_rgba8(),
        color::parse_color("OKLCH(45.9% 0.167 350.36 / 0.7)")
            .unwrap()
            .to_alpha_color::<Srgb>()
            .to_rgba8()
    );
}

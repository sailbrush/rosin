use super::util::*;

#[test]
fn selector_matching() {
    // Simple Wildcard
    let styles = apply_css_to_tree("* { background-color: green; }", two_child_tree);
    assert_eq!(styles[0].background_color, color::palette::css::GREEN);
    assert_eq!(styles[1].background_color, color::palette::css::GREEN);
    assert_eq!(styles[2].background_color, color::palette::css::GREEN);

    // Wildcard Descendant
    let styles = apply_css_to_tree(".parent * { background-color: green; }", two_child_tree);
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, color::palette::css::GREEN);
    assert_eq!(styles[2].background_color, color::palette::css::GREEN);

    // Wildcard Direct Descendant
    let styles = apply_css_to_tree(".child > * { background-color: green; }", |s, ui| {
        ui.node().style_sheet(s).classes("parent").children(|ui| {
            ui.node().classes("child").children(|ui| {
                ui.node().classes("inner_one");
                ui.node().classes("inner_two");
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, Style::default().background_color);
    assert_eq!(styles[2].background_color, color::palette::css::GREEN);
    assert_eq!(styles[3].background_color, color::palette::css::GREEN);

    // Wildcard Descendant with Multiple Levels
    let styles = apply_css_to_tree(".parent * { background-color: green; }", |s, ui| {
        ui.node().style_sheet(s).classes("parent").children(|ui| {
            ui.node().classes("left").children(|ui| {
                ui.node().classes("inner");
            });
            ui.node().classes("right").children(|ui| {
                ui.node().classes("inner");
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, color::palette::css::GREEN);
    assert_eq!(styles[2].background_color, color::palette::css::GREEN);
    assert_eq!(styles[3].background_color, color::palette::css::GREEN);
    assert_eq!(styles[4].background_color, color::palette::css::GREEN);

    // Multiple Classes
    let styles = apply_css_to_tree(".item.special { background-color: yellow; }", |s, ui| {
        ui.node().style_sheet(s).children(|ui| {
            ui.node().classes("item special");
            ui.node().classes("item");
            ui.node().classes("special");
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, color::palette::css::YELLOW);
    assert_eq!(styles[2].background_color, Style::default().background_color);
    assert_eq!(styles[3].background_color, Style::default().background_color);

    // Specific Parent
    let styles = apply_css_to_tree(".container .p { background-color: orange; }", |s, ui| {
        ui.node().style_sheet(s).classes("container").children(|ui| {
            ui.node().classes("p");
            ui.node().children(|ui| {
                ui.node().classes("p");
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, color::palette::css::ORANGE);
    assert_eq!(styles[2].background_color, Style::default().background_color);
    assert_eq!(styles[3].background_color, color::palette::css::ORANGE);

    // Direct Children
    let styles = apply_css_to_tree(".parent > .p { background-color: pink; }", |s, ui| {
        ui.node().style_sheet(s).classes("parent").children(|ui| {
            ui.node().classes("p");
            ui.node().classes("span").children(|ui| {
                ui.node().classes("p");
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, color::palette::css::PINK);
    assert_eq!(styles[2].background_color, Style::default().background_color);
    assert_eq!(styles[3].background_color, Style::default().background_color);

    // Two Selectors
    let styles = apply_css_to_tree(".left, .right { background-color: cyan; }", two_child_tree);
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, color::palette::css::CYAN);
    assert_eq!(styles[2].background_color, color::palette::css::CYAN);

    // Three Selectors
    let styles = apply_css_to_tree(".parent, .left, .right { background-color: cyan; }", two_child_tree);
    assert_eq!(styles[0].background_color, color::palette::css::CYAN);
    assert_eq!(styles[1].background_color, color::palette::css::CYAN);
    assert_eq!(styles[2].background_color, color::palette::css::CYAN);

    // Multiple Classes - Combination and Order
    let styles = apply_css_to_tree(".item.special, .special.item { background-color: blue; }", |s, ui| {
        ui.node().style_sheet(s).children(|ui| {
            ui.node().classes("item special");
            ui.node().classes("special item");
            ui.node().classes("item");
            ui.node().classes("special");
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, color::palette::css::BLUE);
    assert_eq!(styles[2].background_color, color::palette::css::BLUE);
    assert_eq!(styles[3].background_color, Style::default().background_color);
    assert_eq!(styles[4].background_color, Style::default().background_color);

    // Child Combinator with Multiple Levels
    let styles = apply_css_to_tree(".parent > .child > .inner { background-color: purple; }", |s, ui| {
        ui.node().style_sheet(s).classes("parent").children(|ui| {
            ui.node().classes("child").children(|ui| {
                ui.node().classes("inner");
                ui.node().classes("inner").children(|ui| {
                    ui.node().classes("deep-inner");
                });
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, Style::default().background_color);
    assert_eq!(styles[2].background_color, color::palette::css::PURPLE);
    assert_eq!(styles[3].background_color, color::palette::css::PURPLE);
    assert_eq!(styles[4].background_color, Style::default().background_color);

    // Descendant Combinator with Multiple Classes
    let styles = apply_css_to_tree(".parent .child.special { background-color: lime; }", |s, ui| {
        ui.node().style_sheet(s).classes("parent").children(|ui| {
            ui.node().classes("child special");
            ui.node().classes("child").children(|ui| {
                ui.node().classes("special");
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, color::palette::css::LIME);
    assert_eq!(styles[2].background_color, Style::default().background_color);
    assert_eq!(styles[3].background_color, Style::default().background_color);

    // Class Selector with Multiple Levels and Specific Class
    let styles = apply_css_to_tree(".outer .inner .deep.special { background-color: olive; }", |s, ui| {
        ui.node().style_sheet(s).classes("outer").children(|ui| {
            ui.node().classes("inner").children(|ui| {
                ui.node().classes("deep special");
                ui.node().classes("deep");
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, Style::default().background_color);
    assert_eq!(styles[2].background_color, color::palette::css::OLIVE);
    assert_eq!(styles[3].background_color, Style::default().background_color);

    // Descendant Selector with Multiple Levels and Multiple Classes Combination
    let styles = apply_css_to_tree(".outer .inner.special .deep { background-color: navy; }", |s, ui| {
        ui.node().style_sheet(s).classes("outer").children(|ui| {
            ui.node().classes("inner special").children(|ui| {
                ui.node().classes("deep");
            });
            ui.node().classes("inner").children(|ui| {
                ui.node().classes("special deep");
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, Style::default().background_color);
    assert_eq!(styles[2].background_color, color::palette::css::NAVY);
    assert_eq!(styles[3].background_color, Style::default().background_color);

    // Wildcard with Multiple Levels and Specific Class
    let styles = apply_css_to_tree(".outer * .deep { background-color: teal; }", |s, ui| {
        ui.node().style_sheet(s).classes("outer").children(|ui| {
            ui.node().classes("middle").children(|ui| {
                ui.node().classes("deep");
            });
            ui.node().classes("middle").children(|ui| {
                ui.node().classes("inner").children(|ui| {
                    ui.node().classes("deep");
                });
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, Style::default().background_color);
    assert_eq!(styles[2].background_color, color::palette::css::TEAL);
    assert_eq!(styles[3].background_color, Style::default().background_color);
    assert_eq!(styles[4].background_color, Style::default().background_color);
    assert_eq!(styles[5].background_color, color::palette::css::TEAL);

    // Wildcard Direct Descendant with Multiple Levels
    let styles = apply_css_to_tree(".grandparent > .parent > * { background-color: brown; }", |s, ui| {
        ui.node().style_sheet(s).classes("grandparent").children(|ui| {
            ui.node().classes("parent").children(|ui| {
                ui.node().classes("child");
                ui.node().classes("child").children(|ui| {
                    ui.node().classes("inner");
                });
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, Style::default().background_color);
    assert_eq!(styles[2].background_color, color::palette::css::BROWN);
    assert_eq!(styles[3].background_color, color::palette::css::BROWN);
    assert_eq!(styles[4].background_color, Style::default().background_color);

    // Descendant Selector with Specific Class and Deep Levels
    let styles = apply_css_to_tree(".outer .middle .inner .deep { background-color: magenta; }", |s, ui| {
        ui.node().style_sheet(s).classes("outer").children(|ui| {
            ui.node().classes("middle").children(|ui| {
                ui.node().classes("inner").children(|ui| {
                    ui.node().classes("deep");
                    ui.node().classes("deeper");
                });
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, Style::default().background_color);
    assert_eq!(styles[2].background_color, Style::default().background_color);
    assert_eq!(styles[3].background_color, color::palette::css::MAGENTA);
    assert_eq!(styles[4].background_color, Style::default().background_color);

    // Wildcard with Multiple Levels and Different Classes
    let styles = apply_css_to_tree(".container * .item { background-color: silver; }", |s, ui| {
        ui.node().style_sheet(s).classes("container").children(|ui| {
            ui.node().classes("wrapper").children(|ui| {
                ui.node().classes("item");
                ui.node().classes("item").children(|ui| {
                    ui.node().classes("inner_item");
                });
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, Style::default().background_color);
    assert_eq!(styles[2].background_color, color::palette::css::SILVER);
    assert_eq!(styles[3].background_color, color::palette::css::SILVER);
    assert_eq!(styles[4].background_color, Style::default().background_color);

    // Direct Descendant with Multiple Classes and Levels
    let styles = apply_css_to_tree(".box > .content > .text { background-color: maroon; }", |s, ui| {
        ui.node().style_sheet(s).classes("box").children(|ui| {
            ui.node().classes("content").children(|ui| {
                ui.node().classes("text");
                ui.node().classes("text").children(|ui| {
                    ui.node().classes("sub_text");
                });
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, Style::default().background_color);
    assert_eq!(styles[2].background_color, color::palette::css::MAROON);
    assert_eq!(styles[3].background_color, color::palette::css::MAROON);
    assert_eq!(styles[4].background_color, Style::default().background_color);

    // Combination of Class and Descendant Selectors
    let styles = apply_css_to_tree(".box .content .text, .box .footer .text { background-color: aqua; }", |s, ui| {
        ui.node().style_sheet(s).classes("box").children(|ui| {
            ui.node().classes("content").children(|ui| {
                ui.node().classes("text");
            });
            ui.node().classes("footer").children(|ui| {
                ui.node().classes("text");
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, Style::default().background_color);
    assert_eq!(styles[2].background_color, color::palette::css::AQUA);
    assert_eq!(styles[3].background_color, Style::default().background_color);
    assert_eq!(styles[4].background_color, color::palette::css::AQUA);

    // Deep Nested Structure with Multiple Descendant Selectors
    let styles = apply_css_to_tree(".layer1 .layer2 .layer3 .layer4 .target { background-color: coral; }", |s, ui| {
        ui.node().style_sheet(s).classes("layer1").children(|ui| {
            ui.node().classes("layer2").children(|ui| {
                ui.node().classes("layer3").children(|ui| {
                    ui.node().classes("layer4").children(|ui| {
                        ui.node().classes("target");
                    });
                });
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, Style::default().background_color);
    assert_eq!(styles[2].background_color, Style::default().background_color);
    assert_eq!(styles[3].background_color, Style::default().background_color);
    assert_eq!(styles[4].background_color, color::palette::css::CORAL);

    // Descendant Selector with Nested Wildcard and Class Combinations
    let styles = apply_css_to_tree(".container .item * .sub_item { background-color: khaki; }", |s, ui| {
        ui.node().style_sheet(s).classes("container").children(|ui| {
            ui.node().classes("item").children(|ui| {
                ui.node().classes("middle").children(|ui| {
                    ui.node().classes("sub_item");
                });
                ui.node().classes("inner").children(|ui| {
                    ui.node().classes("sub_item");
                });
            });
        });
    });
    assert_eq!(styles[0].background_color, Style::default().background_color);
    assert_eq!(styles[1].background_color, Style::default().background_color);
    assert_eq!(styles[2].background_color, Style::default().background_color);
    assert_eq!(styles[3].background_color, color::palette::css::KHAKI);
    assert_eq!(styles[4].background_color, Style::default().background_color);
    assert_eq!(styles[5].background_color, color::palette::css::KHAKI);
}

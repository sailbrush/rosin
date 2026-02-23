//! Exposes functions to access global shared parley contexts.

use std::sync::OnceLock;

use kurbo::Point;
use parking_lot::RwLock;
use parley::{FontContext, FontStack, Layout, LayoutContext, style::StyleProperty};
use vello::{
    Scene,
    peniko::{self, Fill},
};

use crate::prelude::*;

/// Returns the global shared [`FontContext`] cache used to load fonts.
pub fn global_font_ctx() -> &'static RwLock<FontContext> {
    static FONT_CONTEXT: OnceLock<RwLock<FontContext>> = OnceLock::new();
    FONT_CONTEXT.get_or_init(|| RwLock::new(FontContext::default()))
}

/// Returns the global shared [`LayoutContext`] used to construct text layouts.
pub fn global_text_layout_ctx() -> &'static RwLock<LayoutContext> {
    static LAYOUT_CONTEXT: OnceLock<RwLock<LayoutContext>> = OnceLock::new();
    LAYOUT_CONTEXT.get_or_init(|| RwLock::new(LayoutContext::new()))
}

/// Layout text according to a node's CSS properties. Breaks lines, but doesn't align.
pub(crate) fn layout_text(style: &FontLayoutStyle, max_width: Option<f32>, text: &str) -> Layout<[u8; 4]> {
    let mut font_cx = global_font_ctx().write();
    let mut layout_cx = global_text_layout_ctx().write();

    let font_size = style.font_size;
    let line_height = match &style.line_height {
        Unit::Auto => 1.0,
        Unit::Em(em) => *em,
        Unit::Percent(pct) => *pct,
        Unit::Px(px) => px / font_size,
        Unit::Stretch(s) => *s,
    };

    let mut builder = layout_cx.ranged_builder(&mut font_cx, text, 1.0, true);
    builder.push_default(StyleProperty::FontStack(FontStack::Source(style.font_family.as_deref().map_or("system-ui", |family| family).into())));
    builder.push_default(StyleProperty::FontWeight(parley::style::FontWeight::new(style.font_weight)));
    builder.push_default(StyleProperty::LetterSpacing(style.letter_spacing.unwrap_or(Unit::Px(0.0)).definite_size(font_size, font_size)));
    builder.push_default(StyleProperty::WordSpacing(style.word_spacing.unwrap_or(Unit::Px(0.0)).definite_size(font_size, font_size)));
    builder.push_default(StyleProperty::LineHeight(parley::LineHeight::FontSizeRelative(line_height)));
    builder.push_default(StyleProperty::FontStyle(style.font_style));
    builder.push_default(StyleProperty::FontWidth(parley::FontWidth::from_ratio(style.font_width)));
    builder.push_default(StyleProperty::FontSize(font_size));

    let mut layout = builder.build(text);
    layout.break_all_lines(max_width);
    layout
}

/// Draw text according to a node's CSS properties at a specified location.
pub(crate) fn draw_text(scene: &mut Scene, style: &Style, origin: Point, layout: &Layout<[u8; 4]>) {
    // TODO - waiting on vello support for blurs
    if let Some(text_shadows) = &style.text_shadow {
        for shadow in text_shadows.iter() {
            for line in layout.lines() {
                for item in line.items() {
                    if let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                        let mut run_x = glyph_run.offset();
                        let run_y = glyph_run.baseline();
                        let font = glyph_run.run().font();

                        scene
                            .draw_glyphs(font)
                            .font_size(style.font_size)
                            .brush(shadow.color.unwrap_or(style.color))
                            .draw(
                                &peniko::Style::Fill(Fill::NonZero),
                                glyph_run.glyphs().map(|parley_glyph| {
                                    let x = run_x + parley_glyph.x + (origin.x as f32) + shadow.offset_x.resolve(style.font_size);
                                    let y = run_y - parley_glyph.y + (origin.y as f32) + shadow.offset_y.resolve(style.font_size);
                                    run_x += parley_glyph.advance;

                                    vello::Glyph { id: parley_glyph.id, x, y }
                                }),
                            );
                    }
                }
            }
        }
    }

    for line in layout.lines() {
        for item in line.items() {
            if let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                let mut run_x = glyph_run.offset();
                let run_y = glyph_run.baseline();
                let font = glyph_run.run().font();

                scene.draw_glyphs(font).font_size(style.font_size).brush(style.color).draw(
                    &peniko::Style::Fill(Fill::NonZero),
                    glyph_run.glyphs().map(|parley_glyph| {
                        let x = run_x + parley_glyph.x + (origin.x as f32);
                        let y = run_y - parley_glyph.y + (origin.y as f32);
                        run_x += parley_glyph.advance;

                        vello::Glyph { id: parley_glyph.id, x, y }
                    }),
                );
            }
        }
    }
}

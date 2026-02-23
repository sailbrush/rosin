use std::{
    borrow::Cow,
    ops::Range,
    time::{Duration, Instant},
};

use crate::{
    accesskit,
    kurbo::{Affine, Point, Rect},
    parley::{
        self, FontStack, FontWeight, FontWidth,
        editing::{Cursor, PlainEditor, Selection},
        layout::Affinity,
        style::StyleProperty,
    },
    peniko::{Fill, Style as PenikoStyle},
    prelude::*,
    vello,
    widgets::widget_styles,
};

// TODO - blinks when composing
//      - scroll offset remains when the text doesn't extend out of the box any more
//      - selection-color

struct TextInputHandler {
    composition: WeakVar<Option<Range<usize>>>,
    cursor_visible: WeakVar<bool>,
    editor: WeakVar<PlainEditor<[u8; 4]>>,
    last_activity: WeakVar<Instant>,
    origin: WeakVar<Point>,
    scroll_offset: WeakVar<f32>,

    text: WeakVar<String>,
    window: WindowHandle,
}

impl InputHandler for TextInputHandler {
    fn len(&self) -> usize {
        self.editor.read().unwrap().raw_text().len()
    }

    fn slice<'a>(&'a self, range: Range<usize>) -> Cow<'a, str> {
        let editor = self.editor.read().unwrap();
        let raw = editor.raw_text();
        if range.end <= raw.len() && raw.is_char_boundary(range.start) && raw.is_char_boundary(range.end) {
            Cow::Owned(raw[range].to_string())
        } else {
            Cow::Owned(String::new())
        }
    }

    fn selection(&self) -> Range<usize> {
        self.editor.read().unwrap().raw_selection().text_range()
    }

    fn set_selection(&mut self, range: Range<usize>) {
        if let Some(mut editor) = self.editor.write() {
            let raw = editor.raw_text();
            if range.end <= raw.len() && raw.is_char_boundary(range.start) && raw.is_char_boundary(range.end) {
                let mut font = global_font_ctx().write();
                let mut layout = global_text_layout_ctx().write();
                editor.driver(&mut font, &mut *layout).select_byte_range(range.start, range.end);
            }
        }
    }

    fn composition_range(&self) -> Option<Range<usize>> {
        self.composition.get().unwrap_or(None)
    }

    fn replace_range(&mut self, range: Range<usize>, text: &str) {
        if let Some(mut editor) = self.editor.write() {
            let mut font = global_font_ctx().write();
            let mut layout = global_text_layout_ctx().write();
            let mut drv = editor.driver(&mut font, &mut *layout);

            let raw = drv.editor.raw_text();
            if range.end <= raw.len() && raw.is_char_boundary(range.start) && raw.is_char_boundary(range.end) {
                drv.select_byte_range(range.start, range.end);

                let sanitized = text.replace(['\n', '\r'], " ");
                drv.insert_or_replace_selection(&sanitized);

                self.text.set(drv.editor.text().to_string());

                self.cursor_visible.set(true);
                self.last_activity.set(Instant::now());
            }
        }
    }

    fn set_composition_range(&mut self, range: Option<Range<usize>>) {
        self.composition.set(range);
    }

    fn bounding_box_for_range(&self, range: Range<usize>) -> Option<Rect> {
        let mut editor = self.editor.write().unwrap();
        let mut font = global_font_ctx().write();
        let mut layout = global_text_layout_ctx().write();
        editor.refresh_layout(&mut font, &mut *layout);

        let raw = editor.raw_text();
        if range.end > raw.len() || !raw.is_char_boundary(range.start) || !raw.is_char_boundary(range.end) {
            return None;
        }

        let mut driver = editor.driver(&mut font, &mut *layout);

        // If range is empty, we want the cursor geometry
        let bb = if range.start == range.end {
            let old_range = driver.editor.raw_selection().text_range();

            // Temporarily select to get geometry
            driver.select_byte_range(range.start, range.end);
            let rect = driver.editor.cursor_geometry(1.5);
            driver.select_byte_range(old_range.start, old_range.end);
            rect
        } else {
            let layout = driver.layout();
            let start = Selection::from_byte_index(layout, range.start, Affinity::Downstream);
            let end = Selection::from_byte_index(layout, range.end, Affinity::Downstream).focus();

            let mut union_bb: Option<parley::BoundingBox> = None;
            start.extend(end).geometry_with(layout, |bb, _| {
                if let Some(u) = union_bb.as_mut() {
                    if bb.x0 < u.x0 {
                        u.x0 = bb.x0;
                    }
                    if bb.y0 < u.y0 {
                        u.y0 = bb.y0;
                    }
                    if bb.x1 > u.x1 {
                        u.x1 = bb.x1;
                    }
                    if bb.y1 > u.y1 {
                        u.y1 = bb.y1;
                    }
                } else {
                    union_bb = Some(bb);
                }
            });
            union_bb
        };

        let origin = self.origin.get().unwrap_or(Point::ZERO);
        let scroll = self.scroll_offset.get().unwrap_or(0.0);

        if range.start == range.end {
            let caret = bb
                .map(|b| Rect::new(origin.x + b.x0 - scroll as f64, origin.y + b.y0, origin.x + b.x1 - scroll as f64, origin.y + b.y1))
                .unwrap_or_else(|| Rect::new(origin.x - scroll as f64, origin.y, origin.x - scroll as f64 + 1.0, origin.y + 16.0));

            return Some(caret);
        }

        bb.map(|b| Rect::new(origin.x + b.x0 - scroll as f64, origin.y + b.y0, origin.x + b.x1 - scroll as f64, origin.y + b.y1))
    }

    fn hit_test_point(&self, point: Point) -> Option<Cursor> {
        let mut editor = self.editor.write().unwrap();
        let mut font = global_font_ctx().write();
        let mut layout = global_text_layout_ctx().write();

        editor.refresh_layout(&mut font, &mut *layout);

        let origin = self.origin.get().unwrap_or(Point::ZERO);
        let scroll = self.scroll_offset.get().unwrap_or(0.0);

        let layout_ref = editor.layout(&mut font, &mut *layout);

        let x = (point.x - origin.x + scroll as f64) as f32;
        let y = (point.y - origin.y) as f32;

        Some(Selection::from_point(layout_ref, x, y).focus())
    }

    fn handle_action(&mut self, action: Action) -> bool {
        if self.composition.get().unwrap_or(None).is_some() {
            match action {
                Action::Cut
                | Action::Paste
                | Action::InsertNewLine
                | Action::InsertTab
                | Action::InsertBacktab
                | Action::Delete(_)
                | Action::Move(_)
                | Action::MoveSelecting(_) => return false,
                _ => {}
            }
        }

        let mut editor = self.editor.write().unwrap();
        let mut font = global_font_ctx().write();
        let mut layout = global_text_layout_ctx().write();
        let mut driver = editor.driver(&mut font, &mut *layout);

        let handled = match action {
            Action::Cancel => {
                self.composition.set(None);
                return true;
            }
            Action::Copy | Action::Cut => {
                let selection = driver.editor.raw_selection();
                if !selection.is_collapsed() {
                    let text = &driver.editor.raw_text()[selection.text_range()];
                    self.window.set_clipboard_text(text);
                    if action == Action::Cut {
                        driver.delete_selection();
                    }
                }
                true
            }
            Action::Paste => {
                if let Some(text) = self.window.get_clipboard_text() {
                    let sanitized = text.replace(['\n', '\r'], " ");
                    driver.insert_or_replace_selection(&sanitized);
                }
                true
            }
            Action::Move(mv) => {
                use HorizontalDirection::*;
                match mv {
                    Movement::Grapheme(Left) => driver.move_left(),
                    Movement::Grapheme(Right) => driver.move_right(),
                    Movement::Word(Left) => driver.move_word_left(),
                    Movement::Word(Right) => driver.move_word_right(),
                    Movement::Line(Left) => driver.move_to_line_start(),
                    Movement::Line(Right) => driver.move_to_line_end(),
                    Movement::Document(Left) => driver.move_to_line_start(),
                    Movement::Document(Right) => driver.move_to_line_end(),
                    _ => {}
                }
                true
            }
            Action::MoveSelecting(mv) => {
                use HorizontalDirection::*;
                match mv {
                    Movement::Grapheme(Left) => driver.select_left(),
                    Movement::Grapheme(Right) => driver.select_right(),
                    Movement::Word(Left) => driver.select_word_left(),
                    Movement::Word(Right) => driver.select_word_right(),
                    Movement::Line(Left) => driver.select_to_line_start(),
                    Movement::Line(Right) => driver.select_to_line_end(),
                    Movement::Document(Left) => driver.select_to_line_start(),
                    Movement::Document(Right) => driver.select_to_line_end(),
                    _ => {}
                }
                true
            }
            Action::Delete(mv) => {
                use HorizontalDirection::*;
                match mv {
                    Movement::Grapheme(Left) => driver.backdelete(),
                    Movement::Grapheme(Right) => driver.delete(),
                    Movement::Word(Left) => driver.backdelete_word(),
                    Movement::Word(Right) => driver.delete_word(),
                    Movement::Line(Left) => {
                        driver.select_to_line_start();
                        driver.delete_selection();
                    }
                    Movement::Line(Right) => {
                        driver.select_to_line_end();
                        driver.delete_selection();
                    }
                    Movement::Document(Left) => {
                        driver.select_to_text_start();
                        driver.delete_selection();
                    }
                    Movement::Document(Right) => {
                        driver.select_to_text_end();
                        driver.delete_selection();
                    }
                    _ => {}
                }
                true
            }
            Action::Select(SelectionUnit::All) => {
                driver.select_all();
                true
            }
            Action::Select(SelectionUnit::Word) => {
                if let Some(bb) = driver.editor.cursor_geometry(1.0) {
                    driver.select_word_at_point(bb.x0 as f32, bb.y0 as f32);
                }
                true
            }
            _ => false,
        };

        if handled {
            self.text.set(driver.editor.text().to_string());
            self.cursor_visible.set(true);
            self.last_activity.set(Instant::now());
        }

        handled
    }
}

pub struct TextBox {
    composition: Var<Option<Range<usize>>>,
    cursor_visible: Var<bool>,
    editor: Var<PlainEditor<[u8; 4]>>,
    last_activity: Var<Instant>,
    origin: Var<Point>,
    scroll_offset: Var<f32>,

    layout_config: Var<Option<FontLayoutStyle>>,
    text_version: Var<Option<u64>>,
}

impl Default for TextBox {
    fn default() -> Self {
        let mut editor = PlainEditor::<[u8; 4]>::new(14.0);
        editor.set_quantize(false);
        editor.set_scale(1.0);
        editor.set_width(None);

        Self {
            composition: Var::new(None),
            cursor_visible: Var::new(false),
            editor: Var::new(editor),
            last_activity: Var::new(Instant::now()),
            origin: Var::new(Point::ZERO),
            scroll_offset: Var::new(0.0),

            layout_config: Var::new(None),
            text_version: Var::new(None),
        }
    }
}

impl TextBox {
    pub fn view<'a, S>(&self, ui: &'a mut Ui<S, WindowHandle>, id: NodeId, text: WeakVar<String>) -> &'a mut Ui<S, WindowHandle> {
        let cursor_visible_var = self.cursor_visible.downgrade();
        let editor_var = self.editor.downgrade();
        let composition_var = self.composition.downgrade();
        let origin_var = self.origin.downgrade();
        let scroll_offset_var = self.scroll_offset.downgrade();
        let layout_config_var = self.layout_config.downgrade();
        let text_version_var = self.text_version.downgrade();
        let last_activity_var = self.last_activity.downgrade();

        let run_id = id!(id);

        ui.node()
            .id(id)
            .classes("text-box")
            .style_sheet(widget_styles())
            .event(On::PointerEnter, |_, ctx| {
                ctx.platform().set_cursor(CursorType::Text);
            })
            .event(On::PointerLeave, |_, ctx| {
                ctx.platform().set_cursor(CursorType::Default);
            })
            .event(On::PointerDown, move |_, ctx| {
                ctx.set_focus(ctx.id());

                let Some(pointer) = ctx.pointer() else {
                    return;
                };

                if let Some(mut editor) = editor_var.write() {
                    let mut font_ctx = global_font_ctx().write();
                    let mut layout_ctx = global_text_layout_ctx().write();

                    let pos = pointer.viewport_pos;

                    let origin = origin_var.get().unwrap_or(Point::ZERO);
                    let scroll = scroll_offset_var.get().unwrap_or(0.0);

                    let mut driver = editor.driver(&mut font_ctx, &mut *layout_ctx);

                    let x = (pos.x - origin.x + scroll as f64) as f32;
                    let y = (pos.y - origin.y) as f32;

                    match pointer.count {
                        2 => driver.select_word_at_point(x, y),
                        3 => driver.select_all(),
                        _ => driver.move_to_point(x, y),
                    }
                }
                ctx.begin_pointer_capture();
            })
            .event(On::PointerMove, move |_, ctx| {
                if !ctx.is_pointer_captured() {
                    return;
                }

                let Some(pos) = ctx.local_pointer_pos() else {
                    return;
                };

                if let Some(mut editor) = editor_var.write() {
                    let mut font_ctx = global_font_ctx().write();
                    let mut layout_ctx = global_text_layout_ctx().write();

                    let origin = origin_var.get().unwrap_or(Point::ZERO);
                    let scroll = scroll_offset_var.get().unwrap_or(0.0);

                    let mut driver = editor.driver(&mut font_ctx, &mut *layout_ctx);

                    let x = (pos.x - origin.x + scroll as f64) as f32;
                    let y = (pos.y - origin.y) as f32;

                    driver.extend_selection_to_point(x, y);
                }
            })
            .event(On::PointerUp, move |_, ctx| {
                ctx.end_pointer_capture();
            })
            .event(On::Focus, move |_, ctx| {
                cursor_visible_var.set(true);
                ctx.platform().timer(ctx.id(), Duration::from_millis(600));

                ctx.platform().set_input_handler(
                    ctx.id(),
                    TextInputHandler {
                        window: ctx.platform().clone(),
                        text,
                        editor: editor_var,
                        composition: composition_var,
                        origin: origin_var,
                        scroll_offset: scroll_offset_var,
                        cursor_visible: cursor_visible_var,
                        last_activity: last_activity_var,
                    },
                );
            })
            .event(On::Blur, move |_, ctx| {
                cursor_visible_var.set(false);
                composition_var.set(None);
                ctx.end_pointer_capture();
                ctx.platform().release_input_handler();
            })
            .event(On::Keyboard, |_, ctx| {
                let Some(ev) = ctx.keyboard() else { return };

                if ev.state != KeyState::Down {
                    return;
                }

                if ev.key == Key::Named(NamedKey::Tab) {
                    if ev.modifiers.is_empty() {
                        ctx.focus_next();
                    } else if ev.modifiers.shift() {
                        ctx.focus_previous();
                    }
                }
            })
            .event(On::Timer, move |_, ctx| {
                if !ctx.is_focused() {
                    return;
                }

                let now = Instant::now();
                let last = last_activity_var.get().unwrap_or(now);

                if now.duration_since(last) < Duration::from_millis(300) {
                    cursor_visible_var.set(true);
                } else {
                    cursor_visible_var.set(!cursor_visible_var.get().unwrap_or(false));
                }

                ctx.platform().timer(ctx.id(), Duration::from_millis(600));
            })
            .on_canvas(move |_, ctx| {
                let style = ctx.style;
                let bounds = ctx.padding_box();

                let Some(mut editor) = editor_var.write() else {
                    return;
                };
                let mut scroll_offset = scroll_offset_var.get().unwrap_or(0.0);

                let local_origin = Point::new(
                    ctx.style.child_left.definite_size(ctx.style.font_size, bounds.width() as f32).into(),
                    ctx.style.child_top.definite_size(ctx.style.font_size, bounds.height() as f32).into(),
                );

                origin_var.set(local_origin);

                // Sync text on version change
                let is_composing = composition_var.get().unwrap_or(None).is_some();
                let current_version = text.get_version();
                let last_version = text_version_var.get().flatten();

                if current_version != last_version {
                    if let Some(app_text) = text.get() {
                        let visible_end = app_text.find(['\n', '\r']).unwrap_or(app_text.len());
                        let visible_slice = &app_text[..visible_end];
                        let raw = editor.raw_text();

                        if !is_composing && (raw.len() != visible_slice.len() || editor.text() != visible_slice) {
                            editor.set_text(visible_slice);
                        }
                    }
                    text_version_var.set(current_version);
                }

                // Apply Styles
                let current_config = style.get_font_layout_style();

                let config_changed = layout_config_var.get().map(|last| last != Some(current_config.clone())).unwrap_or(true);
                if config_changed {
                    let styles = editor.edit_styles();
                    let fallback = "'Hiragino Sans', 'Hiragino Kaku Gothic ProN', 'Hiragino Kaku Gothic Pro', \
                    'Yu Gothic', 'YuGothic', 'Meiryo', system-ui, sans-serif";
                    let family = style.font_family.as_deref().unwrap_or(fallback);
                    styles.insert(StyleProperty::FontStack(FontStack::Source(Cow::Owned(family.to_string()))));
                    styles.insert(StyleProperty::FontSize(style.font_size));
                    styles.insert(StyleProperty::FontWeight(FontWeight::new(style.font_weight)));
                    styles.insert(StyleProperty::FontStyle(style.font_style));
                    styles.insert(StyleProperty::FontWidth(FontWidth::from_ratio(style.font_width)));

                    layout_config_var.set(Some(current_config));
                }

                let visible_width = bounds.width() as f32;
                let visible_height = bounds.height() as f32;

                let selection = *editor.raw_selection();
                let compose = composition_var.get().unwrap_or(None);

                let mut font_ctx = global_font_ctx().write();
                let mut layout_ctx = global_text_layout_ctx().write();

                let mut driver = editor.driver(&mut font_ctx, &mut *layout_ctx);

                driver.layout();

                // Handle Scrolling
                let cursor_bb = driver.editor.cursor_geometry(1.5);
                let padding = 10.0f32;
                let target_x = if let Some(bb) = cursor_bb { bb.x1 as f32 } else { 0.0 };

                if target_x > scroll_offset + visible_width - padding {
                    scroll_offset = target_x - visible_width + padding;
                } else if target_x < scroll_offset + padding {
                    scroll_offset = (target_x - padding).max(0.0);
                }

                let layout_ref = driver.layout();

                let total_width = layout_ref.width();
                let max_scroll = (total_width - visible_width + padding).max(0.0);

                if scroll_offset > max_scroll && target_x < max_scroll {
                    scroll_offset = max_scroll;
                }

                scroll_offset_var.set(scroll_offset);

                // ---------- Render ----------
                let margin = style.font_size;
                let view_min_x = scroll_offset - margin;
                let view_max_x = scroll_offset + visible_width + margin;
                let view_max_y = visible_height + margin;

                // Selection Rect
                if ctx.is_focused && !selection.is_collapsed() {
                    selection.geometry_with(layout_ref, |bb, _line_idx| {
                        let r = Rect::new(
                            local_origin.x + bb.x0 - scroll_offset as f64,
                            local_origin.y + bb.y0,
                            local_origin.x + bb.x1 - scroll_offset as f64,
                            local_origin.y + bb.y1,
                        );
                        let clip = r.intersect(bounds);
                        if !clip.is_zero_area() {
                            ctx.scene.fill(Fill::NonZero, Affine::IDENTITY, style.selection_background, None, &clip);
                        }
                    });
                }

                // Composition Underline
                if let Some(comp) = compose {
                    let selection = Selection::from_byte_index(layout_ref, comp.start, Affinity::Downstream)
                        .extend(Selection::from_byte_index(layout_ref, comp.end, Affinity::Downstream).focus());

                    selection.geometry_with(layout_ref, |bb, _| {
                        let y = (local_origin.y + bb.y1 - 1.0).round();
                        let r = Rect::new(local_origin.x + bb.x0 - scroll_offset as f64, y, local_origin.x + bb.x1 - scroll_offset as f64, y + 1.0);
                        let clip = r.intersect(bounds);
                        if !clip.is_zero_area() {
                            ctx.scene.fill(Fill::NonZero, Affine::IDENTITY, style.color, None, &clip);
                        }
                    });
                }

                // Text Rendering
                let base_x = local_origin.x as f32;
                let base_y = local_origin.y as f32;

                for line in layout_ref.lines() {
                    let metrics = line.metrics();

                    // Vertical culling
                    let line_bottom = metrics.baseline + metrics.descent;
                    if line_bottom < 0.0 {
                        continue;
                    }
                    let line_top = metrics.baseline - metrics.ascent;
                    if line_top > view_max_y {
                        break;
                    }

                    let mut run_x = 0.0;

                    for run in line.runs() {
                        let run_start = run_x;
                        let run_end = run_start + run.advance();
                        run_x = run_end;

                        if run_start > view_max_x {
                            break; // everything else is to the right
                        }
                        if run_end < view_min_x {
                            continue; // this whole run is to the left
                        }

                        let font = run.font();
                        let font_size = run.font_size();
                        let baseline = metrics.baseline;

                        let glyph_iter = run
                            .visual_clusters()
                            .flat_map(|c| c.glyphs())
                            .scan(run_start, |pen_x, g| {
                                let gx = *pen_x + g.x;
                                *pen_x += g.advance;
                                Some((g, gx))
                            })
                            .skip_while(|(g, gx)| *gx + g.advance < view_min_x)
                            .take_while(|(_g, gx)| *gx < view_max_x)
                            .map(|(g, gx)| vello::Glyph {
                                id: g.id,
                                x: base_x + gx - scroll_offset,
                                y: base_y + baseline + g.y,
                            });

                        ctx.scene
                            .draw_glyphs(font)
                            .font_size(font_size)
                            .brush(style.color)
                            .draw(&PenikoStyle::Fill(Fill::NonZero), glyph_iter);
                    }
                }

                // Render Cursor
                if ctx.is_focused
                    && cursor_visible_var.get().unwrap_or(false)
                    && let Some(bb) = cursor_bb
                {
                    let r = Rect::new(
                        local_origin.x + bb.x0 - scroll_offset as f64,
                        local_origin.y + bb.y0,
                        local_origin.x + bb.x1 - scroll_offset as f64,
                        local_origin.y + bb.y1,
                    );
                    let clip = r.intersect(bounds);
                    if !clip.is_zero_area() {
                        ctx.scene.fill(Fill::NonZero, Affine::IDENTITY, style.color, None, &clip);
                    }
                }
            })
            .on_accessibility(move |_s, ax| {
                // Expose as an editable single-line text field.
                ax.node.set_role(accesskit::Role::TextInput);
                ax.node.add_action(accesskit::Action::Focus);
                ax.node.add_action(accesskit::Action::SetTextSelection);
                ax.node.add_action(accesskit::Action::SetValue);
                ax.node.add_action(accesskit::Action::ReplaceSelectedText);

                // Value and selection come from the editor.
                let Some(editor) = editor_var.read() else {
                    return;
                };
                let raw = editor.raw_text();

                // Keep the value in sync with the editable text.
                ax.node.set_value(raw);

                // Convert editor byte indices -> AccessKit character indices.
                let byte_to_char_index = |s: &str, byte_idx: usize| -> usize {
                    let mut b = 0usize;
                    let mut ci = 0usize;
                    for ch in s.chars() {
                        if b >= byte_idx {
                            break;
                        }
                        b += ch.len_utf8();
                        ci += 1;
                    }
                    ci
                };

                let sel = editor.raw_selection().text_range();
                let anchor_ci = byte_to_char_index(raw, sel.start);
                let focus_ci = byte_to_char_index(raw, sel.end);

                ax.node.set_text_selection(accesskit::TextSelection {
                    anchor: accesskit::TextPosition {
                        node: run_id.into(),
                        character_index: anchor_ci,
                    },
                    focus: accesskit::TextPosition {
                        node: run_id.into(),
                        character_index: focus_ci,
                    },
                });
            })
            .event(On::AccessibilityAction, move |_, ctx| {
                let Some(req) = ctx.action_request() else { return };

                let char_index_to_byte = |s: &str, char_idx: usize| -> usize { s.chars().take(char_idx).map(|ch| ch.len_utf8()).sum() };

                match req.action {
                    accesskit::Action::SetTextSelection => {
                        let Some(accesskit::ActionData::SetTextSelection(sel)) = req.data.as_ref() else {
                            return;
                        };

                        // Only accept selections that target our run node.
                        if sel.anchor.node != run_id.into() || sel.focus.node != run_id.into() {
                            return;
                        }

                        let Some(mut editor) = editor_var.write() else { return };
                        let raw = editor.raw_text().to_string();

                        let a = char_index_to_byte(&raw, sel.anchor.character_index);
                        let f = char_index_to_byte(&raw, sel.focus.character_index);
                        let start = a.min(f);
                        let end = a.max(f);

                        let mut font = global_font_ctx().write();
                        let mut layout = global_text_layout_ctx().write();
                        editor.driver(&mut font, &mut *layout).select_byte_range(start, end);

                        text.set(editor.text().to_string());
                        cursor_visible_var.set(true);
                        last_activity_var.set(Instant::now());
                        ctx.emit_change();
                    }
                    accesskit::Action::SetValue => {
                        let Some(accesskit::ActionData::Value(v)) = req.data.as_ref() else { return };
                        let sanitized = v.replace(['\n', '\r'], " ");

                        let Some(mut editor) = editor_var.write() else { return };
                        editor.set_text(&sanitized);

                        text.set(editor.text().to_string());
                        cursor_visible_var.set(true);
                        last_activity_var.set(Instant::now());
                        ctx.emit_change();
                    }
                    accesskit::Action::ReplaceSelectedText => {
                        let Some(accesskit::ActionData::Value(v)) = req.data.as_ref() else { return };
                        let sanitized = v.replace(['\n', '\r'], " ");

                        let Some(mut editor) = editor_var.write() else { return };
                        let mut font = global_font_ctx().write();
                        let mut layout = global_text_layout_ctx().write();
                        editor.driver(&mut font, &mut *layout).insert_or_replace_selection(&sanitized);

                        text.set(editor.text().to_string());
                        cursor_visible_var.set(true);
                        last_activity_var.set(Instant::now());
                        ctx.emit_change();
                    }
                    _ => {}
                }
            })
            .children(move |ui| {
                // AccessKit text requires selection positions to target a Role::TextRun node.
                // This node is semantic-only.
                ui.node().id(run_id).on_accessibility(move |_s, ax| {
                    ax.node.set_role(accesskit::Role::TextRun);

                    let Some(editor) = editor_var.read() else {
                        return;
                    };
                    let raw = editor.raw_text();
                    ax.node.set_value(raw);

                    let mut lengths: Vec<u8> = Vec::with_capacity(raw.chars().count());
                    for ch in raw.chars() {
                        lengths.push(ch.len_utf8() as u8);
                    }
                    ax.node.set_character_lengths(lengths);
                });
            })
    }
}

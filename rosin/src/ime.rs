//! Types for interacting with the platform's text input APIs.

use std::{borrow::Cow, ops::Range};

use crate::kurbo::{Point, Rect};
use crate::prelude::*;

/// A lock on the text document allowing the platform backend to query state and apply edits.
///
/// Text input is a bidirectional conversation: the application provides the OS with document state
/// and geometry, and the OS requests edits (typing, paste, IME composition).
///
/// ## Coordinate systems
/// All points and rectangles in this trait are expressed in Viewport Coordinates
/// (logical pixels relative to the top-left of the viewport).
///
/// ## Units for indices and ranges
/// All indices and ranges are in UTF-8 byte offsets, matching Rust `str`/`String` indexing rules.
///
/// Platform backends that use UTF-16 (macOS/Windows) must convert using
/// `utf8_range_utf16_len` and `utf16_range_to_utf8_range`.
///
/// ## Range validity contract
/// Unless explicitly stated otherwise:
/// - `start <= end`
/// - `end <= len()`
/// - `start` and `end` must be UTF-8 char boundaries [`str::is_char_boundary`].
///
/// Implementations may choose to further clamp/adjust to extended grapheme cluster boundaries for
/// user-facing editing operations, but must do so deterministically.
pub trait InputHandler {
    /// Returns `true` if the document contains no text.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total length of the document in UTF-8 bytes.
    fn len(&self) -> usize;

    /// Returns a view of the document text for `range`.
    ///
    /// Implementations should return a borrowed `&str` when possible, but may allocate and return an owned string.
    fn slice<'a>(&'a self, range: Range<usize>) -> Cow<'a, str>;

    /// Returns the current user selection or caret position, as a UTF-8 byte range.
    ///
    /// A caret is represented as an empty range.
    fn selection(&self) -> Range<usize>;

    /// Updates the selection (caret / highlighted range), as a UTF-8 byte range.
    ///
    /// This is often called by the platform backend in response to IME requests to move the caret
    /// or update the selection during composition, without modifying the document text.
    fn set_selection(&mut self, selection: Range<usize>);

    /// Returns the range of text currently being composed (marked / pre-edit), if any.
    ///
    /// Returns `None` if no IME composition is active.
    fn composition_range(&self) -> Option<Range<usize>>;

    /// Sets or clears the active composition (marked / pre-edit) range.
    ///
    /// `Some(range)` means an IME composition session is active and currently applies to `range`.
    /// `None` means there is no active composition.
    ///
    /// This is typically driven by the platform backend while handling IME "marked text" APIs
    /// (macOS `setMarkedText`/`unmarkText`, Windows TSF composition events).
    ///
    /// Both `range.start` and `range.end` must be `<= self.len()`, and should be char boundaries.
    ///
    /// If you clamp/adjust to extended grapheme cluster boundaries, do so deterministically and
    /// keep internal invariants consistent with `selection()` and subsequent edit operations.
    fn set_composition_range(&mut self, range: Option<Range<usize>>);

    /// Replaces text in the document with `text`.
    ///
    /// This method is the primitive edit operation used for:
    /// - normal typing, paste, and deletions (committed text), and
    /// - IME updates (pre-edit / marked text).
    ///
    /// Calling this method does not automatically clear or finalize IME composition.
    /// The caller is responsible for deciding whether this edit represents a composition update,
    /// a commit/finalization, or an edit outside composition and will update `composition_range()`
    /// and `selection()` accordingly.
    fn replace_range(&mut self, range: Range<usize>, text: &str);

    /// Performs a semantic action.
    ///
    /// Returns `true` if the action was handled, `false` if it should be handled as a normal keypress.
    fn handle_action(&mut self, action: Action) -> bool;

    /// Returns the text position closest to `point` (Viewport Coordinates).
    ///
    /// Used for hit-testing / mouse placement / some IME queries.
    fn hit_test_point(&self, point: Point) -> Option<Cursor>;

    /// Returns the bounding box of the text in `range` (Viewport Coordinates).
    ///
    /// If `range` has length 0, this should return the bounding box of the caret at that position.
    ///
    /// Used by the OS to position IME candidate windows and system dictionaries.
    fn bounding_box_for_range(&self, range: Range<usize>) -> Option<Rect>;

    /// Returns the number of UTF-16 code units in the provided UTF-8 range.
    ///
    /// This is used to map Rust UTF-8 byte ranges to macOS/Windows UTF-16 selection ranges.
    ///
    /// The default implementation performs an O(N) scan. Implementors backed by accelerated data
    /// structures (like ropes) should override this to provide O(log N) lookups.
    ///
    /// Returns `None` if the range is invalid or not on UTF-8 char boundaries.
    fn utf8_range_utf16_len(&self, range: Range<usize>) -> Option<usize> {
        if range.start > range.end || range.end > self.len() {
            return None;
        }

        let slice = self.slice(0..self.len());

        // Ensure boundaries align with UTF-8 char boundaries.
        if !slice.is_char_boundary(range.start) || !slice.is_char_boundary(range.end) {
            return None;
        }

        Some(slice[range].chars().map(|c| c.len_utf16()).sum())
    }

    /// Converts a UTF-16 code-unit range into a UTF-8 byte range in the document.
    ///
    /// This is used to map macOS/Windows requests back to Rust UTF-8 byte ranges.
    ///
    /// The default implementation performs an O(N) scan from the start of the document.
    /// Implementors backed by accelerated data structures (like ropes) should override this to
    /// provide O(log N) lookups.
    ///
    /// Returns `None` if the UTF-16 range is invalid or extends past the document end.
    fn utf16_range_to_utf8_range(&self, range: Range<usize>) -> Option<Range<usize>> {
        if range.start > range.end {
            return None;
        }

        if range.start == 0 && range.end == 0 {
            return Some(0..0);
        }

        let text = self.slice(0..self.len());
        let eof_byte = text.len();

        let mut current_utf16 = 0usize;
        let mut byte_start: Option<usize> = None;
        let mut byte_end: Option<usize> = None;

        for (byte_idx, ch) in text.char_indices() {
            let next_utf16 = current_utf16 + ch.len_utf16();
            let next_byte = byte_idx + ch.len_utf8();

            if byte_start.is_none() {
                if range.start == current_utf16 {
                    byte_start = Some(byte_idx);
                } else if range.start == next_utf16 {
                    byte_start = Some(next_byte);
                }
            }

            if byte_end.is_none() {
                if range.end == current_utf16 {
                    byte_end = Some(byte_idx);
                } else if range.end == next_utf16 {
                    byte_end = Some(next_byte);
                }
            }

            current_utf16 = next_utf16;

            if byte_start.is_some() && byte_end.is_some() {
                break;
            }
        }

        // Handle EOF positions
        if byte_start.is_none() && range.start == current_utf16 {
            byte_start = Some(eof_byte);
        }
        if byte_end.is_none() && range.end == current_utf16 {
            byte_end = Some(eof_byte);
        }

        // UTF-16 range extends past document end
        if range.end > current_utf16 {
            return None;
        }

        match (byte_start, byte_end) {
            (Some(s), Some(e)) if s <= e => Some(s..e),
            _ => None,
        }
    }
}

/// A semantic editing action triggered by user input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Moves the caret based on the specified movement rules.
    Move(Movement),
    /// Moves the caret while extending the current selection.
    MoveSelecting(Movement),
    /// Deletes text defined by the specified movement.
    Delete(Movement),
    /// Selects a specific semantic unit of text.
    Select(SelectionUnit),
    /// Inserts a line break at the current position.
    InsertNewLine,
    /// Inserts a tab character or indent.
    InsertTab,
    /// Removes a tab character or unindents the current line/selection.
    InsertBacktab,
    /// Copies the current selection to the system clipboard.
    Copy,
    /// Copies the current selection to the system clipboard and deletes the selected text.
    Cut,
    /// Inserts content from the system clipboard at the current cursor position.
    Paste,
    /// Cancels the current operation.
    Cancel,
}

impl Action {
    pub fn edits_text(&self) -> bool {
        matches!(self, Action::Delete(_) | Action::InsertNewLine | Action::InsertTab | Action::InsertBacktab | Action::Cut | Action::Paste)
    }
}

/// Defines the granularity and direction of a cursor movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Movement {
    /// Movement by a single visual character (grapheme cluster).
    Grapheme(HorizontalDirection),
    /// Movement by a word boundary.
    Word(HorizontalDirection),
    /// Movement to the start or end of the current line.
    Line(HorizontalDirection),
    /// Movement to the start or end of the current paragraph.
    Paragraph(HorizontalDirection),
    /// Movement to the start or end of the entire document.
    Document(HorizontalDirection),
    /// Vertical movement across lines or pages.
    Vertical(VerticalDirection),
}

/// A horizontal direction relative to the text layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalDirection {
    /// Visual left (or logical backward).
    Left,
    /// Visual right (or logical forward).
    Right,
}

/// A vertical direction relative to the text layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalDirection {
    /// Moves up one line.
    Up,
    /// Moves down one line.
    Down,
    /// Moves up by one viewport height.
    PageUp,
    /// Moves down by one viewport height.
    PageDown,
}

/// A semantic unit of text to be selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionUnit {
    /// The word surrounding the current cursor position.
    Word,
    /// The line containing the current cursor position.
    Line,
    /// The paragraph containing the current cursor position.
    Paragraph,
    /// The entire document text.
    All,
}

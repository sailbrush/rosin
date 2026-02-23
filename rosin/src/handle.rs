//! Types related to interacting with the platform window handle.

use std::{any::Any, time::Duration};

use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle as RWHWindowHandle};

use crate::kurbo::{Point, Size};
use crate::prelude::*;

/// A handle to the application window.
///
/// Returned by [`EventCtx::platform`].
///
/// This handle is cheaply cloneable and provides a thread-safe interface
/// to interact with window properties and the OS.
#[derive(Clone)]
pub struct WindowHandle(pub(crate) crate::platform::handle::WindowHandle);

impl HasWindowHandle for WindowHandle {
    fn window_handle(&self) -> Result<RWHWindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}

impl HasDisplayHandle for WindowHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.0.display_handle()
    }
}

impl WindowHandle {
    /// Sets the active IME handler for this window.
    ///
    /// An [`On::Change`] event will be sent to the node id provided when the text is edited.
    pub fn set_input_handler(&self, id: impl Into<Option<NodeId>>, handler: impl InputHandler + Send + Sync + 'static) {
        self.0.set_input_handler(id.into(), Some(Box::new(handler)))
    }

    /// Removes the current IME handler.
    pub fn release_input_handler(&self) {
        self.0.set_input_handler(None, None)
    }

    /// Returns the size of the window's content area in physical pixels.
    pub fn get_physical_size(&self) -> Size {
        self.0.get_physical_size()
    }

    /// Returns the size of the window's content area in logical units, scaled by the system's DPI factor.
    pub fn get_logical_size(&self) -> Size {
        self.0.get_logical_size()
    }

    /// Returns the screen-space position of the window's outer top-left corner (in logical units, Y-down).
    pub fn get_position(&self) -> Point {
        self.0.get_position()
    }

    /// Returns the current state of the window (Minimized, Maximized, or Normal).
    pub fn get_window_state(&self) -> WindowState {
        self.0.get_window_state()
    }

    /// Returns true if the window is currently focused.
    pub fn is_active(&self) -> bool {
        self.0.is_active()
    }

    /// Makes the window gain focus and brings it to the front.
    pub fn activate(&self) {
        self.0.activate()
    }

    /// Makes the window lose focus.
    pub fn deactivate(&self) {
        self.0.deactivate()
    }

    /// Sets the window's top-level menu bar.
    pub fn set_menu(&self, menu: impl Into<Option<MenuDesc>>) {
        self.0.set_menu(menu)
    }

    /// Shows a context menu at the provided logical coordinates.
    ///
    /// [`CommandId`]s are sent to the Node.
    pub fn show_context_menu(&self, id: Option<NodeId>, menu: MenuDesc, pos: Point) {
        self.0.show_context_menu(id, menu, pos)
    }

    /// Creates a new window with the given description.
    ///
    /// `S` must be the same type as the state value passed to [`AppLauncher::run()`].
    pub fn create_window<S: Any + Sync + 'static>(&self, desc: &WindowDesc<S>) {
        self.0.create_window(desc)
    }

    /// Request that the window close.
    pub fn request_close(&self) {
        self.0.request_close()
    }

    /// Request that the application exit by stopping the main event loop.
    ///
    /// This causes [`AppLauncher::run()`] to return for clean up.
    pub fn request_exit(&self) {
        self.0.request_exit()
    }

    /// Sets the maximum logical size that the window's content area can be resized to.
    ///
    /// A value of `None` disables the restriction.
    pub fn set_max_size(&self, size: Option<impl Into<Size>>) {
        self.0.set_max_size(size)
    }

    /// Sets the minimum logical size that the window's content area can be resized to.
    ///
    /// A value of `None` disables the restriction.
    pub fn set_min_size(&self, size: Option<impl Into<Size>>) {
        self.0.set_min_size(size)
    }

    /// Sets the screen-space position of the window's outer top-left corner (in logical units, Y-down).
    pub fn set_position(&self, position: impl Into<Point>) {
        self.0.set_position(position)
    }

    /// Sets whether the window can be resized by the user.
    pub fn set_resizable(&self, resizable: bool) {
        self.0.set_resizable(resizable)
    }

    /// Sets the logical size of a window's content area.
    pub fn set_size(&self, size: impl Into<Size>) {
        self.0.set_size(size)
    }

    /// Sets the text displayed in the title bar of the window.
    pub fn set_title(&self, title: impl Into<String>) {
        self.0.set_title(title)
    }

    /// Minimizes the window to the taskbar or dock.
    pub fn minimize(&self) {
        self.0.minimize()
    }

    /// Maximizes the window to fill the screen.
    pub fn maximize(&self) {
        self.0.maximize()
    }

    /// Restores the window to its previous size and position from a minimized or maximized state.
    pub fn restore(&self) {
        self.0.restore()
    }

    /// Sets the appearance of the mouse cursor.
    pub fn set_cursor(&self, cursor: CursorType) {
        self.0.set_cursor(cursor)
    }

    /// Hides the mouse cursor.
    pub fn hide_cursor(&self) {
        self.0.hide_cursor()
    }

    /// Shows the mouse cursor if it was previously hidden.
    pub fn unhide_cursor(&self) {
        self.0.unhide_cursor()
    }

    /// Copies the provided text to the system clipboard.
    pub fn set_clipboard_text(&self, text: &str) {
        self.0.set_clipboard_text(text)
    }

    /// Retrieves the current text content of the system clipboard, if any.
    pub fn get_clipboard_text(&self) -> Option<String> {
        self.0.get_clipboard_text()
    }

    /// Opens a URL in the user's default web browser or appropriate system application.
    pub fn open_url(&self, url: &str) {
        self.0.open_url(url)
    }

    /// Opens the system file picker.
    ///
    /// The results will be returned to the node in an [`On::FileDialog`] event.
    pub fn open_file_dialog(&self, id: Option<NodeId>, options: FileDialogOptions) {
        self.0.open_file_dialog(id, options)
    }

    /// Opens the system 'Save As' dialog.
    ///
    /// The results will be returned to the node in an [`On::FileDialog`] event.
    pub fn save_file_dialog(&self, id: Option<NodeId>, options: FileDialogOptions) {
        self.0.save_file_dialog(id, options)
    }

    /// Queues an [`On::Timer`] event for the node after `delay`.
    pub fn timer(&self, id: Option<NodeId>, delay: Duration) {
        self.0.timer(id, delay)
    }

    /// Opens a default native modal dialog.
    pub fn alert(&self, title: &str, details: &str) {
        self.0.alert::<CommandId>(None, None, title, details, &[])
    }

    /// Opens a native modal dialog with an optional image and custom buttons.
    ///
    /// When one of the buttons is clicked, an [`On::Command`] event will be fired on the provided `node` with the associated [`CommandId`].
    pub fn alert_custom<C>(&self, node: Option<NodeId>, png_bytes: Option<&'static [u8]>, title: &str, details: &str, options: &[(&'static str, C)])
    where
        C: Into<CommandId> + Copy,
    {
        self.0.alert(node, png_bytes, title, details, options)
    }
}

/// Visual representation of a cursor.
///
/// Refer to the table at <https://developer.mozilla.org/en-US/docs/Web/CSS/cursor#values>
#[derive(Copy, Clone, Debug, Default)]
pub enum CursorType {
    #[default]
    Default,

    // Links & status
    ContextMenu,
    Help,
    Pointer,

    // Selection
    Cell,
    Crosshair,
    Text,
    VerticalText,

    // Drag & drop
    Alias,
    Copy,
    Move,
    NotAllowed,
    Grab,
    Grabbing,

    // Resizing & scrolling
    ColResize,
    RowResize,
    NResize,
    EResize,
    SResize,
    WResize,
    NEResize,
    NWResize,
    SEResize,
    SWResize,
    EWResize,
    NSResize,
    NESWResize,
    NWSEResize,

    // Zooming
    ZoomIn,
    ZoomOut,
}

use std::{any::Any, time::Duration};

use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle as RWHWindowHandle};

use crate::{
    kurbo::{Point, Size},
    prelude::*,
};

pub(crate) struct WindowHandle {}

impl Clone for WindowHandle {
    fn clone(&self) -> Self {
        Self {}
    }
}

impl HasWindowHandle for WindowHandle {
    fn window_handle(&self) -> Result<RWHWindowHandle<'_>, HandleError> {
        Err(HandleError::Unavailable)
    }
}

impl HasDisplayHandle for WindowHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Err(HandleError::Unavailable)
    }
}

impl WindowHandle {
    pub fn set_input_handler(&self, _id: Option<NodeId>, _handler: Option<Box<dyn InputHandler + Send + Sync>>) {}

    pub fn get_logical_size(&self) -> Size {
        Size::ZERO
    }

    pub fn get_physical_size(&self) -> Size {
        Size::ZERO
    }

    pub fn get_position(&self) -> Point {
        Point::ZERO
    }

    pub fn get_window_state(&self) -> WindowState {
        WindowState::Normal
    }

    pub fn is_active(&self) -> bool {
        true
    }

    pub fn activate(&self) {}

    pub fn deactivate(&self) {}

    pub fn set_menu(&self, _menu: impl Into<Option<MenuDesc>>) {}

    pub fn show_context_menu(&self, _node: Option<NodeId>, _menu: MenuDesc, _pos: Point) {}

    pub fn create_window<S: Any + Sync + 'static>(&self, _desc: &WindowDesc<S>) {}

    pub fn request_close(&self) {}

    pub fn request_exit(&self) {}

    pub fn set_max_size(&self, _size: Option<impl Into<Size>>) {}

    pub fn set_min_size(&self, _size: Option<impl Into<Size>>) {}

    pub fn set_position(&self, _position: impl Into<Point>) {}

    pub fn set_resizable(&self, _resizeable: bool) {}

    pub fn set_size(&self, _size: impl Into<Size>) {}

    pub fn set_title(&self, _title: impl Into<String>) {}

    pub fn minimize(&self) {}

    pub fn maximize(&self) {}

    pub fn restore(&self) {}

    pub fn set_cursor(&self, _cursor: CursorType) {}

    pub fn hide_cursor(&self) {}

    pub fn unhide_cursor(&self) {}

    pub fn set_clipboard_text(&self, _text: &str) {}

    pub fn get_clipboard_text(&self) -> Option<String> {
        None
    }

    pub fn open_url(&self, _url: &str) {}

    pub fn open_file_dialog(&self, _node: Option<NodeId>, _options: FileDialogOptions) {}

    pub fn save_file_dialog(&self, _node: Option<NodeId>, _options: FileDialogOptions) {}

    pub fn timer(&self, _node: Option<NodeId>, _delay: Duration) {}

    pub fn alert<C>(&self, _node: Option<NodeId>, _png_bytes: Option<&'static [u8]>, _title: &str, _details: &str, _options: &[(&'static str, C)])
    where
        C: Into<CommandId> + Copy,
    {
    }
}

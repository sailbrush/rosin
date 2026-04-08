use crate::linux::util::cursor_icon_to_shape;
use crate::linux::wayland::WaylandWindow;
use crate::{
    kurbo::{Point, Size},
    prelude::*,
};
use raw_window_handle::RawDisplayHandle;
use raw_window_handle::WaylandDisplayHandle;
use raw_window_handle::WaylandWindowHandle;
use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle as RWHWindowHandle};
use rosin_core::parking_lot::RwLock;
use std::borrow::Borrow;
use std::option;
use std::ptr::NonNull;
use std::sync::Arc;
use std::{any::Any, time::Duration};
use wayland_client::Proxy;
use zbus::blocking::Connection;
pub(crate) struct InputHandlerVars {
    pub(crate) id: Option<NodeId>,
    pub(crate) handler: Option<Box<dyn InputHandler + Send + Sync>>,
}

pub(crate) struct WindowHandle {
    pub(crate) wayland_handle: Option<Arc<WaylandWindow>>,
    pub(crate) input_handler: Arc<RwLock<InputHandlerVars>>,
}

impl Clone for WindowHandle {
    fn clone(&self) -> Self {
        Self {
            wayland_handle: self.wayland_handle.clone(),
            input_handler: self.input_handler.clone(),
        }
    }
}

impl HasWindowHandle for WindowHandle {
    fn window_handle(&self) -> Result<RWHWindowHandle<'_>, HandleError> {
        unsafe {
            Ok(RWHWindowHandle::borrow_raw(raw_window_handle::RawWindowHandle::Wayland(WaylandWindowHandle::new(
                NonNull::new(self.wayland_handle.as_ref().unwrap().surface.id().as_ptr() as *mut _).unwrap(),
            ))))
        }
    }
}

impl HasDisplayHandle for WindowHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        unsafe {
            Ok(DisplayHandle::borrow_raw(RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
                NonNull::new(self.wayland_handle.as_ref().unwrap().conn.as_ref().unwrap().backend().display_ptr() as *mut _).unwrap(),
            ))))
        }
    }
}

impl WindowHandle {
    pub fn set_input_handler(&self, _id: Option<NodeId>, _handler: Option<Box<dyn InputHandler + Send + Sync>>) {
        let clone: &RwLock<InputHandlerVars> = self.input_handler.borrow();
        let mut input_handle = clone.write();
        input_handle.handler = _handler;
        input_handle.id = _id;
    }

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

    pub fn set_max_size(&self, size: Option<impl Into<Size>>) {
        if size.is_some() {
            let s = size.unwrap().into();
            self.wayland_handle.clone().unwrap().xdg_toplevel.set_max_size(s.width as i32, s.height as i32);
        }
    }

    pub fn set_min_size(&self, size: Option<impl Into<Size>>) {
        if size.is_some() {
            let s = size.unwrap().into();
            self.wayland_handle.clone().unwrap().xdg_toplevel.set_min_size(s.width as i32, s.height as i32);
        }
    }

    pub fn set_position(&self, _position: impl Into<Point>) {}

    pub fn set_resizable(&self, _resizeable: bool) {}

    pub fn set_size(&self, _size: impl Into<Size>) {}

    pub fn set_title(&self, title: impl Into<String>) {
        self.wayland_handle.clone().unwrap().xdg_toplevel.set_title(title.into());
    }

    pub fn minimize(&self) {
        self.wayland_handle.clone().unwrap().xdg_toplevel.set_minimized();
    }

    pub fn maximize(&self) {
        self.wayland_handle.clone().unwrap().xdg_toplevel.set_maximized();
    }

    pub fn restore(&self) {
        self.wayland_handle.clone().unwrap().xdg_toplevel.unset_maximized();
    }

    pub fn set_cursor(&self, cursor: CursorType) {
        self.wayland_handle
            .as_ref()
            .unwrap()
            .pointer_shape
            .as_ref()
            .unwrap()
            .set_shape(self.wayland_handle.as_ref().unwrap().last_pointer_serial, cursor_icon_to_shape(cursor));
    }

    pub fn hide_cursor(&self) {}

    pub fn unhide_cursor(&self) {}

    pub fn set_clipboard_text(&self, _text: &str) {}

    pub fn get_clipboard_text(&self) -> Option<String> {
        None
    }
    // TODO: make safer?
    pub fn open_url(&self, url: &str) {
        use std::process::Command;
        let mut cmd = Command::new("xdg-open");
        cmd.arg(url);
        let _ = cmd.spawn();
    }

    pub fn open_file_dialog(&self, _node: Option<NodeId>, options: FileDialogOptions) {
        use crate::linux::util::file_dialog_to_open;
        let conn = Connection::session().expect("msg");
        let proxy =
            zbus::blocking::Proxy::new(&conn, "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.FileChooser")
                .unwrap();
        let loc: String = proxy.call("OpenFile", &("", "", file_dialog_to_open(options))).unwrap();
    }

    pub fn save_file_dialog(&self, _node: Option<NodeId>, options: FileDialogOptions) {
        use crate::linux::util::file_dialog_to_save;
        let conn = Connection::session().expect("msg");
        let proxy =
            zbus::blocking::Proxy::new(&conn, "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.FileChooser")
                .unwrap();
        let loc: String = proxy.call("SaveFile", &("", "", file_dialog_to_save(options))).unwrap();
    }

    pub fn timer(&self, _node: Option<NodeId>, _delay: Duration) {}

    pub fn alert<C>(&self, _node: Option<NodeId>, _png_bytes: Option<&'static [u8]>, _title: &str, _details: &str, _options: &[(&'static str, C)])
    where
        C: Into<CommandId> + Copy,
    {
    }
}

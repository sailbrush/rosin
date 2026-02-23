//! Types related to the behavior of an application window.

use std::sync::Arc;

use crate::kurbo::{Point, Size};
use crate::prelude::*;

/// The current presentation state of a window.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WindowState {
    Minimized,
    Maximized,
    Normal,
}

/// A description of a window to be created.
pub struct WindowDesc<S: 'static> {
    pub(crate) viewfn: ViewFn<S, WindowHandle>,
    pub(crate) wgpufn: Option<WgpuFn<S>>,
    pub(crate) title: Option<Arc<str>>,
    pub(crate) menu: Option<MenuDesc>,
    pub(crate) size: Size,
    pub(crate) min_size: Option<Size>,
    pub(crate) max_size: Option<Size>,
    pub(crate) resizeable: bool,
    pub(crate) position: Option<Point>,
    pub(crate) close_button: bool,
    pub(crate) minimize_button: bool,
    pub(crate) maximize_button: bool,
}

impl<S> Clone for WindowDesc<S> {
    fn clone(&self) -> Self {
        Self {
            viewfn: self.viewfn,
            wgpufn: self.wgpufn,
            title: self.title.clone(),
            menu: self.menu.clone(),
            size: self.size,
            min_size: self.min_size,
            max_size: self.max_size,
            resizeable: self.resizeable,
            position: self.position,
            close_button: self.close_button,
            minimize_button: self.minimize_button,
            maximize_button: self.maximize_button,
        }
    }
}

impl<S> WindowDesc<S> {
    /// Creates a new [`WindowDesc`] with the specified view function.
    ///
    /// Use the [`callback`] macro to convert a function item with the signature `fn(&S, &mut Ui<S, WindowHandle>)`.
    ///
    /// **Example:**
    /// ```ignore
    /// fn my_view(state: &State, ui: &mut Ui<State, WindowHandle>) { ... }
    ///
    /// let desc = WindowDesc::new(callback!(my_view));
    /// ```
    pub fn new(viewfn: impl Into<ViewFn<S, WindowHandle>>) -> Self {
        Self {
            viewfn: viewfn.into(),
            wgpufn: None,
            title: None,
            menu: None,
            size: (500.0, 500.0).into(),
            min_size: None,
            max_size: None,
            resizeable: true,
            position: None,
            close_button: true,
            minimize_button: true,
            maximize_button: true,
        }
    }

    /// The provided function will be called to give the app an opportunity to render to the WGPU surface before the UI.
    ///
    /// Use the [`callback`] macro to convert a function item with the signature `fn(&S, &mut WgpuCtx<'_>)`.
    ///
    /// **Example:**
    /// ```ignore
    /// fn wgpu_callback(state: &State, ctx: &mut WgpuCtx<'_>) { ... }
    ///
    /// let desc = WindowDesc::new(callback!(my_view))
    ///     .wgpu(callback!(wgpu_callback));
    /// ```
    pub fn wgpu(mut self, wgpufn: impl Into<WgpuFn<S>>) -> Self {
        self.wgpufn = Some(wgpufn.into());
        self
    }

    /// Sets the text to be displayed in the title bar of the window.
    ///
    /// Defaults to a blank title bar.
    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the window's menu.
    pub fn menu(mut self, menu: MenuDesc) -> Self {
        self.menu = Some(menu);
        self
    }

    /// Sets the initial logical size of the window's content area.
    ///
    /// Defaults to 500 x 500
    pub fn size(mut self, width: u64, height: u64) -> Self {
        self.size = (width as f64, height as f64).into();
        self
    }

    /// Sets the minimum logical size that the window's content area can be resized to.
    pub fn min_size(mut self, width: u64, height: u64) -> Self {
        self.min_size = Some((width as f64, height as f64).into());
        self
    }

    /// Sets the maximum logical size that the window's content area can be resized to.
    pub fn max_size(mut self, width: u64, height: u64) -> Self {
        self.max_size = Some((width as f64, height as f64).into());
        self
    }

    /// Sets whether the window can be resized by the user.
    ///
    /// Defaults to `true`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizeable = resizable;
        self
    }

    /// Sets the initial position of the window.
    ///
    /// If not specified, the window will be centered on the screen.
    ///
    /// On macOS, the origin is the bottom-left corner of the screen.
    ///
    /// On Windows and Linux, the origin is the top-left corner of the screen.
    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.position = Some((x, y).into());
        self
    }

    /// Sets whether the window has a visible close button.
    ///
    /// Defaults to `true`.
    pub fn close_button(mut self, visible: bool) -> Self {
        self.close_button = visible;
        self
    }

    /// Sets whether the window has a visible minimize button.
    ///
    /// Defaults to `true`.
    pub fn minimize_button(mut self, visible: bool) -> Self {
        self.minimize_button = visible;
        self
    }

    /// Sets whether the window has a visible maximize button.
    ///
    /// Defaults to `true`.
    pub fn maximize_button(mut self, visible: bool) -> Self {
        self.maximize_button = visible;
        self
    }
}

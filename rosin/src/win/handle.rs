use std::{any::Any, num::NonZeroIsize, sync::Arc, time::Duration};

use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle as RWHWindowHandle};

use crate::{
    kurbo::{Point, Size},
    platform::view::{RosinView, ThreadLockedView},
    prelude::*, win::view::f64_to_i32,
};

pub(crate) struct WindowHandle {
    /// TODO make thread safe - force to only be accesable on the main thread
    /// Most likely to some extra struct
    ///  - Do the check internally or use the same MainKey kinda initialization as the macos version?
    pub(in crate::win) view: Arc<ThreadLockedView>,
}

impl Clone for WindowHandle {
    fn clone(&self) -> Self {
        Self { view: self.view.clone() }
    }
}

impl HasWindowHandle for WindowHandle {
    fn window_handle(&self) -> Result<RWHWindowHandle<'_>, HandleError> {
        self.view
            .try_on_thread(|view| {
                let raw_ptr = view.hwnd().0;
                let handle = Win32WindowHandle::new(NonZeroIsize::new(raw_ptr as isize).ok_or(HandleError::Unavailable)?);

                unsafe { Ok(RWHWindowHandle::borrow_raw(RawWindowHandle::Win32(handle))) }
            })
            .expect("RawWindowHandle must be requested from the main thread")
    }
}

impl HasDisplayHandle for WindowHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::windows())
    }
}

impl WindowHandle {
    pub(crate) fn new(view: RosinView) -> WindowHandle {
        Self {
            view: Arc::new(ThreadLockedView::new(view)),
        }
    }

    pub fn set_input_handler(&self, _id: Option<NodeId>, _handler: Option<Box<dyn InputHandler + Send + Sync>>) {}

    pub fn get_logical_size(&self) -> Size {
        use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
        use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
        use windows::Win32::UI::HiDpi::GetDpiForWindow;
        use windows::Win32::Foundation::RECT;

        self.view.block_on_thread(
            // uses the formula `DIPs = pixels / (DPI / USER_DEFAULT_SCREEN_DPI)` but refactored a bit
            |view| {
                let mut rect: RECT = Default::default();

                let desktop_dpi = unsafe {
                    GetDpiForWindow(GetDesktopWindow()) as f64
                };

                let view_dpi = unsafe {
                    GetDpiForWindow(view.hwnd()) as f64
                };

                let scale_factor = desktop_dpi / view_dpi;

                unsafe {
                    // SAFETY:
                    //  - view.hwnd() is a valid window handle
                    //  - &raw mut is a poitner to a valid RECT position in memory
                    if let Err(err) = GetWindowRect(view.hwnd(), &raw mut rect) {
                        eprintln!("`get_logical_size` {err}: {err:#?}");
                    }
                }

                Size::new(
                    (rect.right - rect.left) as f64 * scale_factor,
                    (rect.bottom - rect.top) as f64 * scale_factor,
                )
            }
        )
    }

    pub fn get_physical_size(&self) -> Size {
        use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
        use windows::Win32::Foundation::RECT;

        self.view.block_on_thread(
            |view| {
                let mut rect: RECT = Default::default();

                unsafe {
                    // SAFETY:
                    //  - view.hwnd() is a valid window handle
                    //  - &raw mut is a poitner to a valid RECT position in memory
                    if let Err(err) = GetWindowRect(view.hwnd(), &raw mut rect) {
                        eprintln!("`get_logical_size` {err}: {err:#?}");
                    }
                }

                Size::new(
                    (rect.right - rect.left) as f64,
                    (rect.bottom - rect.top) as f64,
                )
            }
        )
    }

    pub fn get_position(&self) -> Point {
        Point::ZERO
    }

    pub fn get_window_state(&self) -> WindowState {
        WindowState::Normal
    }

    pub fn is_active(&self) -> bool {
        // IsWindowEnabled
        true
    }

    pub fn activate(&self) {
        // EnableWindow (set to false)
    }

    pub fn deactivate(&self) {
        // SetActiveWindow
    }

    pub fn set_menu(&self, _menu: impl Into<Option<MenuDesc>>) {
        /* call DrawMenuBar to redraw the menu bar */
    }

    pub fn show_context_menu(&self, _node: Option<NodeId>, _menu: MenuDesc, _pos: Point) {}

    pub fn create_window<S: Any + Sync + 'static>(&self, _desc: &WindowDesc<S>) {}

    pub fn request_close(&self) {}

    pub fn request_exit(&self) {}

    pub fn set_max_size(&self, size: Option<impl Into<Size>>) {
        let size = size.map(Into::into);

        self.view.queue_on_thread(
            move |view| {
                use crate::platform::view::ViewStateSize;

                let Some(view_state) = view.get_view_state() else {
                    return
                };

                let max_size = match size {
                    Some(size) => ViewStateSize {
                        x: f64_to_i32(size.width),
                        y: f64_to_i32(size.height),
                    },
                    None => ViewStateSize::default_max(),
                };

                unsafe {
                    // SAFETY: view_state points to a valid ViewState
                    (*view_state.as_ptr()).size_bounds.max = max_size
                }
            }
        )
    }

    pub fn set_min_size(&self, size: Option<impl Into<Size>>) {
        let size = size.map(Into::into);

        self.view.queue_on_thread(
            move |view| {
                use crate::platform::view::ViewStateSize;

                let Some(view_state) = view.get_view_state() else {
                    return
                };

                let min_size = match size {
                    Some(size) => ViewStateSize {
                        x: f64_to_i32(size.width),
                        y: f64_to_i32(size.height),
                    },
                    None => ViewStateSize::default_min(),
                };

                unsafe {
                    // SAFETY: view_state points to a valid ViewState
                    (*view_state.as_ptr()).size_bounds.min = min_size
                }
            }
        )
    }

    pub fn set_position(&self, position: impl Into<Point>) {
        use crate::platform::view::f64_to_i32;
        use windows::Win32::UI::WindowsAndMessaging::{SWP_ASYNCWINDOWPOS, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos};

        let position = position.into();

        unsafe {
            // TEMP SAFETY: read the **WIN32 THREAD SAFETY** comment from `.../rosin/src/win/mod.rs`
            // SAFETY: SWP_ASYNCWINDOWPOS guarantees thread safety
            self.view.try_on_trust(move |view| {
                // SAFETY:
                //  - view.hwnd() is a valid handle
                //  - None is a valid optional handle
                //  - SWP_ASYNCWINDOWPOS, SWP_NOSIZE and SWP_NOZORDER are a valid value when xor'ed
                let _res = SetWindowPos(
                    view.hwnd(),
                    None,
                    f64_to_i32(position.x),
                    f64_to_i32(position.y),
                    0, 0,
                    SWP_ASYNCWINDOWPOS | SWP_NOSIZE | SWP_NOZORDER
                );

                #[cfg(debug_assertions)]
                if let Err(err) = _res {
                    eprintln!("`set_size` failed to change the size for `{hwnd:?}` \"{err}\": {err:#?}", hwnd = view.hwnd())
                }
            })
        }
    }

    pub fn set_resizable(&self, resizeable: bool) {
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowLongPtrW,
            GetWindowLongPtrW,
            GWL_STYLE,
            WS_SIZEBOX,
        };

        unsafe {
            // TEMP SAFETY: read the **WIN32 THREAD SAFETY** comment from `.../rosin/src/win/mod.rs`
            self.view.try_on_trust(
                move |view| {
                    let style = GetWindowLongPtrW(view.hwnd(), GWL_STYLE);
                    
                    match resizeable {
                        true => SetWindowLongPtrW(view.hwnd(), GWL_STYLE, style | WS_SIZEBOX.0 as isize),
                        false => SetWindowLongPtrW(view.hwnd(), GWL_STYLE, style & !(WS_SIZEBOX.0 as isize)),
                    };
                }
            )
        }
    }

    pub fn set_size(&self, size: impl Into<Size>) {
        use crate::platform::view::f64_to_i32;
        use windows::Win32::UI::WindowsAndMessaging::{SWP_ASYNCWINDOWPOS, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos};

        let size = size.into();

        unsafe {
            // TEMP SAFETY: read the **WIN32 THREAD SAFETY** comment from `.../rosin/src/win/mod.rs`
            // SAFETY: SWP_ASYNCWINDOWPOS guarantees thread safety
            self.view.try_on_trust(move |view| {
                // SAFETY:
                //  - view.hwnd() is a valid handle
                //  - None is a valid optional handle
                //  - SWP_ASYNCWINDOWPOS, SWP_NOMOVE and SWP_NOZORDER are a valid value when xor'ed
                let _res =
                    SetWindowPos(view.hwnd(), None, 0, 0, f64_to_i32(size.width), f64_to_i32(size.height), SWP_ASYNCWINDOWPOS | SWP_NOMOVE | SWP_NOZORDER);

                #[cfg(debug_assertions)]
                if let Err(err) = _res {
                    eprintln!("`set_size` failed to change the size for `{hwnd:?}` \"{err}\": {err:#?}", hwnd = view.hwnd())
                }
            })
        }
    }

    // NOTE: maybe add a `set_position_and_size` method for microptimizations?

    pub fn set_title(&self, _title: impl Into<String>) {}

    pub fn minimize(&self) {
        unsafe {
            self.view
                .try_on_trust(
                    |view| {
                        use windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE;
    
                        // SAFETY: all given values are valid
                        let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindowAsync(view.hwnd(), SW_MINIMIZE);
                    }
                );
        }
    }

    pub fn maximize(&self) {
        unsafe {
            self.view
                .try_on_trust(
                    |view| {
                        use windows::Win32::UI::WindowsAndMessaging::SW_MAXIMIZE;

                        // SAFETY: all given values are valid
                        let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindowAsync(view.hwnd(), SW_MAXIMIZE);
                    }
                );
        }
    }

    pub fn restore(&self) {
        unsafe {
            self.view
                .try_on_trust(
                    |view| {
                        use windows::Win32::UI::WindowsAndMessaging::SW_RESTORE;

                        // SAFETY: all given values are valid
                        let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindowAsync(view.hwnd(), SW_RESTORE);
                    }
                );
        }
    }

    pub fn set_cursor(&self, cursor: CursorType) {
        use windows::Win32::UI::WindowsAndMessaging::{HCURSOR, SetCursor};

        self.view.queue_on_thread(
            move |_| {
                let handle_cursor: Option<HCURSOR> = match cursor {
                    CursorType::Default => None,

                    /*
                    CursorType::ContextMenu => ...,
                    CursorType::Help => ...,
                    CursorType::Pointer => ...,

                    CursorType::Cell => ...,
                    CursorType::Crosshair => ...,
                    CursorType::Text => ...,
                    CursorType::VerticalText => ...,

                    CursorType::Alias => ...,
                    CursorType::Copy => ...,
                    CursorType::Move => ...,
                    CursorType::NotAllowed => ...,
                    CursorType::Grab => ...,
                    CursorType::Grabbing => ...,

                    CursorType::ColResize => ...,
                    CursorType::RowResize => ...,
                    CursorType::NResize => ...,
                    CursorType::EResize => ...,
                    CursorType::SResize => ...,
                    CursorType::WResize => ...,
                    CursorType::NEResize => ...,
                    CursorType::NWResize => ...,
                    CursorType::SEResize => ...,
                    CursorType::SWResize => ...,
                    CursorType::EWResize => ...,
                    CursorType::NSResize => ...,
                    CursorType::NESWResize => ...,
                    CursorType::NWSEResize => ...,

                    CursorType::ZoomIn => ...,
                    CursorType::ZoomOut => ...,
                    */

                    _ => todo!("handling the {cursor:?} cursor type."),
                };

                unsafe {
                    SetCursor(handle_cursor);
                }
            }
        )
    }

    pub fn hide_cursor(&self) {
        use windows::Win32::UI::WindowsAndMessaging::ShowCursor;

        self.view.queue_on_thread(
            |_| unsafe {
                ShowCursor(false);
            }
        )
    }

    pub fn unhide_cursor(&self) {
        use windows::Win32::UI::WindowsAndMessaging::ShowCursor;

        self.view.queue_on_thread(
            |_| unsafe {
                ShowCursor(true);
            }
        )
    }

    pub fn set_clipboard_text(&self, _text: &str) {
        /*
        // I believe I am on the right track with this code.
        // I would like to have the ability to requeue code that fails though
        // running low on time though so simply comma comment this out and leave it for later

        use crate::platform::view::{
            utf8_to_utf16,
            utf16_as_pcwstr,
        };

        use windows::Win32::System::{
            DataExchange::{
                OpenClipboard,
                CloseClipboard,
                EmptyClipboard,
                SetClipboardData,
            },

            Ole::CF_UNICODETEXT,
        };

        let utf16 = utf8_to_utf16(text);

        self.view.queue_on_thread(
            |view| {
                unsafe {
                    OpenClipboard(Some(view.hwnd())).ok()?;
                    EmptyClipboard().ok()?;

                    SetClipboardData(CF_UNICODETEXT.0 as u32, /* utf16 -> hmem */)

                    CloseClipboard().ok()?;
                }
            }
        )
        */
    }

    pub fn get_clipboard_text(&self) -> Option<String> {
        use windows::Win32::System::{
            DataExchange::{
                OpenClipboard,
                CloseClipboard,
                GetClipboardData,
            },

            Ole::CF_UNICODETEXT,
        };

        self.view.block_on_thread(
            move |view| {
                unsafe {
                    OpenClipboard(Some(view.hwnd())).ok()?
                }

                let handle = unsafe {
                    GetClipboardData(CF_UNICODETEXT.0 as u32).ok()?
                };

                let mut got_carrige_feed = false;
                let mut idx = 0;
                let win32_str = handle.0 as *mut u16;

                let mut string = String::new();

                while unsafe { *win32_str.add(idx) } != 0 {
                    let c = unsafe {
                        char::from_u32_unchecked(*win32_str.add(idx) as u32)
                    };

                    match c {
                        '\r' => got_carrige_feed = true,
                        '\n' => {
                            got_carrige_feed = false;
                            string.push('\n');
                        }
                        c => {
                            if got_carrige_feed {
                                string.push('\r');
                                got_carrige_feed = false;
                            }
                            string.push(c);
                        },
                    }

                    idx += 1;
                }

                unsafe {
                    CloseClipboard().ok()?
                }

                Some(string)
            }
        )
    }

    pub fn open_url(&self, url: &str) {
        use windows::{
            core::w,

            Win32::UI::{
                WindowsAndMessaging::SW_NORMAL,
                Shell::ShellExecuteW,
            },
        };

        use super::view::{
            utf8_to_utf16,
            utf16_as_pcwstr,
        };

        let url_utf16 = utf8_to_utf16(url);

        self.view.queue_on_thread(
            move |view| {
                let result = unsafe {
                    ShellExecuteW(
                        Some(view.hwnd()),
                        w!("open"),
                        utf16_as_pcwstr(&url_utf16),
                        None,
                        None,
                        SW_NORMAL,
                    )  
                };

                // incase error handling is ever added this link lists all the error codes
                // https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutew
                match result.0 as usize {
                    0..=32 => {
                        let err = unsafe {
                            // SAFETY: The ShellExecuteW function guarantees that
                            //   an error did not occur only if the return code is > 32
                            windows::Win32::Foundation::GetLastError()
                        };
                        eprintln!("`open_url` An error occured! #{code:p} {err:#?}", code = result.0);
                    },
                    33.. => (),
                }
            }
        )

        // https://learn.microsoft.com/en-us/windows/win32/winhttp/iwinhttprequest-open
        // https://learn.microsoft.com/en-us/windows/win32/winhttp/winhttprequest
        // https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Networking/WinHttp/struct.IWinHttpRequest.html
    }

    pub fn open_file_dialog(&self, _node: Option<NodeId>, _options: FileDialogOptions) {}

    pub fn save_file_dialog(&self, _node: Option<NodeId>, _options: FileDialogOptions) {}

    pub fn timer(&self, _node: Option<NodeId>, _delay: Duration) {}

    pub fn alert<C>(&self, _node: Option<NodeId>, _png_bytes: Option<&'static [u8]>, _title: &str, _details: &str, _options: &[(&'static str, C)])
    where
        C: Into<CommandId> + Copy,
    {
    }
}

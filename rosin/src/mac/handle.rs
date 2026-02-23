use std::{
    any::Any,
    cell::OnceCell,
    ffi::c_void,
    ptr::NonNull,
    sync::{Arc, mpsc},
    time::Duration,
};

use block2::RcBlock;
use dispatch2::{DispatchQueue, DispatchTime, MainThreadBound};
use objc2::{AnyThread, DefinedClass, rc::Retained, sel};
use objc2_app_kit::{
    NSAlert, NSApp, NSCursor, NSHorizontalDirections, NSImage, NSModalResponseOK, NSOpenPanel, NSPasteboard, NSPasteboardTypeString, NSSavePanel, NSScreen,
    NSVerticalDirections, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSData, NSPoint, NSSize, NSString, NSURL};
use objc2_uniform_type_identifiers::UTType;
use raw_window_handle::{AppKitWindowHandle, DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle, WindowHandle as RWHWindowHandle};

use crate::{
    kurbo::{Point, Size},
    log::warn,
    mac::{util, window::RosinView},
    prelude::*,
};

pub(crate) struct WindowHandle {
    pub ns_view: Arc<MainThreadBound<Retained<RosinView>>>,
}

impl Clone for WindowHandle {
    fn clone(&self) -> Self {
        Self { ns_view: self.ns_view.clone() }
    }
}

impl HasWindowHandle for WindowHandle {
    fn window_handle(&self) -> Result<RWHWindowHandle<'_>, HandleError> {
        let mtm = MainThreadMarker::new().expect("RawWindowHandle must be requested from the main thread");
        let ns_view = self.ns_view.get(mtm);
        let ns_view_ptr = Retained::as_ptr(ns_view) as *mut c_void;
        let handle = AppKitWindowHandle::new(NonNull::new(ns_view_ptr).ok_or(HandleError::Unavailable)?);

        // SAFETY: The pointer is derived from a valid Retained<RosinView> ensuring validity.
        unsafe { Ok(RWHWindowHandle::borrow_raw(RawWindowHandle::AppKit(handle))) }
    }
}

impl HasDisplayHandle for WindowHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::appkit())
    }
}

impl WindowHandle {
    pub(crate) fn new(mtm: MainThreadMarker, ns_view: Retained<RosinView>) -> crate::platform::handle::WindowHandle {
        Self {
            ns_view: Arc::new(MainThreadBound::new(ns_view, mtm)),
        }
    }

    fn queue_on_main<F>(&self, f: F)
    where
        F: FnOnce(&RosinView, MainThreadMarker) + Send + 'static,
    {
        let ns_view = self.ns_view.clone();
        DispatchQueue::main().exec_async(move || {
            // SAFETY: DispatchQueue::main() guarantees this block is running on the main thread.
            let mtm = unsafe { MainThreadMarker::new_unchecked() };
            let view = ns_view.get(mtm);
            f(view, mtm);
        });
    }

    fn block_on_main<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&RosinView, MainThreadMarker) -> R + Send + 'static,
        R: Send + 'static,
    {
        if let Some(mtm) = MainThreadMarker::new() {
            let view = self.ns_view.get(mtm);
            return f(view, mtm);
        }

        let (tx, rx) = mpsc::channel();
        let ns_view = self.ns_view.clone();

        DispatchQueue::main().exec_async(move || {
            // SAFETY: DispatchQueue::main() guarantees this block is running on the main thread.
            let mtm = unsafe { MainThreadMarker::new_unchecked() };
            let view = ns_view.get(mtm);
            let result = f(view, mtm);
            let _ = tx.send(result);
        });

        rx.recv().expect("Main thread dropped task without returning result")
    }

    pub fn set_input_handler(&self, id: Option<NodeId>, handler: Option<Box<dyn InputHandler + Send + Sync>>) {
        self.queue_on_main(move |view, _| {
            view.set_input_handler(id, handler);
        });
    }

    pub fn get_logical_size(&self) -> Size {
        self.block_on_main(|view, _| {
            let bounds = view.bounds();
            Size::new(bounds.size.width, bounds.size.height)
        })
    }

    pub fn get_physical_size(&self) -> Size {
        self.block_on_main(|view, _| {
            let bounds = view.bounds();
            let backing = view.convertRectToBacking(bounds);
            Size::new(backing.size.width, backing.size.height)
        })
    }

    pub fn get_position(&self) -> Point {
        self.block_on_main(|view, mtm| {
            let Some(window) = view.window() else {
                return Point::ZERO;
            };

            let window_frame = window.frame();

            let window_left = window_frame.origin.x;
            let window_top = window_frame.origin.y + window_frame.size.height;

            // Pick the screen the window is on, fallback to main.
            let Some(screen) = window.screen().or_else(|| NSScreen::mainScreen(mtm)) else {
                // No screen info, fall back to global coordinates.
                return Point::new(window_left, -window_top);
            };

            let screen_frame = screen.frame();
            let screen_left = screen_frame.origin.x;
            let screen_top = screen_frame.origin.y + screen_frame.size.height;

            // Position relative to screen top-left, with Y-down.
            Point::new(window_left - screen_left, screen_top - window_top)
        })
    }

    pub fn get_window_state(&self) -> WindowState {
        self.block_on_main(|view, _| {
            if let Some(window) = view.window() {
                if window.isMiniaturized() {
                    WindowState::Minimized
                } else if window.isZoomed() {
                    WindowState::Maximized
                } else {
                    WindowState::Normal
                }
            } else {
                WindowState::Normal
            }
        })
    }

    pub fn is_active(&self) -> bool {
        self.block_on_main(|view, _| view.window().map(|w| w.isKeyWindow()).unwrap_or(false))
    }

    pub fn activate(&self) {
        self.queue_on_main(|view, mtm| {
            if let Some(window) = view.window() {
                NSApp(mtm).activate();
                window.makeKeyAndOrderFront(None);
            }
        });
    }

    pub fn deactivate(&self) {
        self.queue_on_main(|view, _| {
            if let Some(window) = view.window() {
                window.resignKeyWindow();
            }
        });
    }

    pub fn set_menu(&self, menu: impl Into<Option<MenuDesc>>) {
        let menu = menu.into();
        self.queue_on_main(move |view, _| {
            view.set_main_menu(menu);
        });
    }

    pub fn show_context_menu(&self, node: Option<NodeId>, menu: MenuDesc, pos: Point) {
        let Some(node) = node else {
            return;
        };
        self.queue_on_main(move |view, _| {
            view.show_context_menu(node, menu, pos);
        });
    }

    pub fn create_window<S: Any + Sync + 'static>(&self, desc: &WindowDesc<S>) {
        let desc_boxed = Box::new(desc.clone());
        self.queue_on_main(move |view, mtm| {
            view.ivars().viewport.borrow_mut().create_window(mtm, view, desc_boxed);
        });
    }

    pub fn request_close(&self) {
        self.queue_on_main(|view, _| {
            if let Some(window) = view.window() {
                window.performClose(None);
            }
        });
    }

    pub fn request_exit(&self) {
        self.queue_on_main(|_, mtm| {
            NSApp(mtm).stop(None);
        });
    }

    pub fn set_max_size(&self, size: Option<impl Into<Size>>) {
        let size = size.map(Into::into);
        self.queue_on_main(move |view, _| {
            if let Some(window) = view.window() {
                if let Some(max_size) = size {
                    window.setContentMaxSize(NSSize::new(max_size.width, max_size.height));
                } else {
                    window.setContentMaxSize(NSSize::new(f64::MAX, f64::MAX));
                }
            }
        });
    }

    pub fn set_min_size(&self, size: Option<impl Into<Size>>) {
        let size = size.map(Into::into);
        self.queue_on_main(move |view, _| {
            if let Some(window) = view.window() {
                if let Some(min_size) = size {
                    window.setContentMinSize(NSSize::new(min_size.width, min_size.height));
                } else {
                    window.setContentMinSize(NSSize::ZERO);
                }
            }
        });
    }

    pub fn set_position(&self, position: impl Into<Point>) {
        let position = position.into();

        self.queue_on_main(move |view, mtm| {
            let Some(window) = view.window() else {
                return;
            };

            if let Some(screen) = window.screen().or_else(|| NSScreen::mainScreen(mtm)) {
                let s = screen.frame();

                let screen_left = s.origin.x;
                let screen_top = s.origin.y + s.size.height;

                let global_x = screen_left + position.x;
                let global_y = screen_top - position.y;

                window.setFrameTopLeftPoint(NSPoint::new(global_x, global_y));
            } else {
                window.setFrameTopLeftPoint(NSPoint::new(position.x, -position.y));
            }
        });
    }

    pub fn set_resizable(&self, resizeable: bool) {
        self.queue_on_main(move |view, _| {
            if let Some(window) = view.window() {
                window.setStyleMask(if resizeable {
                    window.styleMask() | NSWindowStyleMask::Resizable
                } else {
                    window.styleMask() & !NSWindowStyleMask::Resizable
                });
            }
        });
    }

    pub fn set_size(&self, size: impl Into<Size>) {
        let size = size.into();

        self.queue_on_main(move |view, _| {
            if let Some(window) = view.window() {
                let old_frame = window.frame();
                let old_content_rect = window.contentRectForFrameRect(old_frame);

                let mut new_content_rect = old_content_rect;
                new_content_rect.size = NSSize::new(size.width, size.height);

                let mut new_frame = window.frameRectForContentRect(new_content_rect);

                // keep the window centered
                new_frame.origin.x = old_frame.origin.x + (old_frame.size.width - new_frame.size.width) / 2.0;
                new_frame.origin.y = old_frame.origin.y + (old_frame.size.height - new_frame.size.height) / 2.0;

                window.setFrame_display_animate(new_frame, true, true);
            }
        });
    }

    pub fn set_title(&self, title: impl Into<String>) {
        let title = title.into();
        self.queue_on_main(move |view, _| {
            if let Some(window) = view.window() {
                window.setTitle(&NSString::from_str(&title));
            }
        });
    }

    pub fn minimize(&self) {
        self.queue_on_main(|view, _| {
            if let Some(window) = view.window() {
                window.miniaturize(None);
            }
        });
    }

    pub fn maximize(&self) {
        self.queue_on_main(|view, _| {
            if let Some(window) = view.window() {
                window.zoom(None);
            }
        });
    }

    pub fn restore(&self) {
        self.queue_on_main(|view, _| {
            if let Some(window) = view.window() {
                if window.isMiniaturized() {
                    window.deminiaturize(None);
                }
                if window.isZoomed() {
                    window.zoom(None);
                }
            }
        });
    }

    pub fn set_cursor(&self, cursor: CursorType) {
        self.queue_on_main(move |_, _| {
            thread_local! {
                static CELL_CURSOR: OnceCell<Retained<NSCursor>> = const { OnceCell::new() };
                static MOVE_CURSOR: OnceCell<Retained<NSCursor>> = const { OnceCell::new() };
            }

            match cursor {
                CursorType::Cell => {
                    // The cursor PDFs have been in this directory on macOS since at least 2013, so hopefully this will continue to be stable.
                    let path = "/System/Library/Frameworks/ApplicationServices.framework/Versions/A/Frameworks/HIServices.framework/Versions/A/Resources/cursors/cell/cursor.pdf";
                    CELL_CURSOR.with(|c| {
                        c.get_or_init(|| {
                            util::load_cursor_from_pdf(path, 8.0, 8.0)
                                .unwrap_or_else(NSCursor::arrowCursor)
                        })
                        .set();
                    });
                }
                CursorType::Move => {
                    let path = "/System/Library/Frameworks/ApplicationServices.framework/Versions/A/Frameworks/HIServices.framework/Versions/A/Resources/cursors/move/cursor.pdf";
                    MOVE_CURSOR.with(|c| {
                        c.get_or_init(|| {
                            util::load_cursor_from_pdf(path, 8.0, 8.0)
                                .unwrap_or_else(NSCursor::arrowCursor)
                        })
                        .set();
                    });
                }
                CursorType::Default => NSCursor::arrowCursor().set(),
                CursorType::ContextMenu => NSCursor::contextualMenuCursor().set(),
                CursorType::Pointer => NSCursor::pointingHandCursor().set(),
                CursorType::Crosshair => NSCursor::crosshairCursor().set(),
                CursorType::Text => NSCursor::IBeamCursor().set(),
                CursorType::VerticalText => NSCursor::IBeamCursorForVerticalLayout().set(),
                CursorType::Alias => NSCursor::dragLinkCursor().set(),
                CursorType::Copy => NSCursor::dragCopyCursor().set(),
                CursorType::NotAllowed => NSCursor::operationNotAllowedCursor().set(),
                CursorType::Grab => NSCursor::openHandCursor().set(),
                CursorType::Grabbing => NSCursor::closedHandCursor().set(),
                CursorType::ColResize => NSCursor::columnResizeCursorInDirections(NSHorizontalDirections::Left | NSHorizontalDirections::Right).set(),
                CursorType::RowResize => NSCursor::rowResizeCursorInDirections(NSVerticalDirections::Up | NSVerticalDirections::Down).set(),
                CursorType::Help => util::set_private_cursor(sel!(_helpCursor)),
                CursorType::NResize => util::set_private_cursor(sel!(_windowResizeNorthCursor)),
                CursorType::EResize => util::set_private_cursor(sel!(_windowResizeEastCursor)),
                CursorType::SResize => util::set_private_cursor(sel!(_windowResizeSouthCursor)),
                CursorType::WResize => util::set_private_cursor(sel!(_windowResizeWestCursor)),
                CursorType::NEResize => util::set_private_cursor(sel!(_windowResizeNorthEastCursor)),
                CursorType::NWResize => util::set_private_cursor(sel!(_windowResizeNorthWestCursor)),
                CursorType::SEResize => util::set_private_cursor(sel!(_windowResizeSouthEastCursor)),
                CursorType::SWResize => util::set_private_cursor(sel!(_windowResizeSouthWestCursor)),
                CursorType::EWResize => util::set_private_cursor(sel!(_windowResizeEastWestCursor)),
                CursorType::NSResize => util::set_private_cursor(sel!(_windowResizeNorthSouthCursor)),
                CursorType::NESWResize => util::set_private_cursor(sel!(_windowResizeNorthEastSouthWestCursor)),
                CursorType::NWSEResize => util::set_private_cursor(sel!(_windowResizeNorthWestSouthEastCursor)),
                CursorType::ZoomIn => util::set_private_cursor(sel!(_zoomInCursor)),
                CursorType::ZoomOut => util::set_private_cursor(sel!(_zoomOutCursor)),
            }
        });
    }

    pub fn hide_cursor(&self) {
        self.queue_on_main(|_, _| NSCursor::hide());
    }

    pub fn unhide_cursor(&self) {
        self.queue_on_main(|_, _| NSCursor::unhide());
    }

    pub fn set_clipboard_text(&self, text: &str) {
        let text_owned = text.to_string();
        self.queue_on_main(move |_, _| {
            let pasteboard = NSPasteboard::generalPasteboard();
            pasteboard.clearContents();
            let ns_str = NSString::from_str(&text_owned);
            // SAFETY: writing string type to pasteboard is standard API usage.
            unsafe {
                pasteboard.setString_forType(&ns_str, NSPasteboardTypeString);
            }
        });
    }

    pub fn get_clipboard_text(&self) -> Option<String> {
        self.block_on_main(|_, _| {
            let pasteboard = NSPasteboard::generalPasteboard();
            // SAFETY: reading string type from pasteboard is standard API usage.
            unsafe { pasteboard.stringForType(NSPasteboardTypeString).map(|s| s.to_string()) }
        })
    }

    pub fn open_url(&self, url: &str) {
        let url = url.to_string();
        self.queue_on_main(move |_, _| {
            let ns_url_str = NSString::from_str(&url);
            if let Some(ns_url) = NSURL::URLWithString(&ns_url_str) {
                NSWorkspace::sharedWorkspace().openURL(&ns_url);
            }
        });
    }

    pub fn open_file_dialog(&self, node: Option<NodeId>, options: FileDialogOptions) {
        let Some(node) = node else {
            return;
        };
        let ns_view = self.ns_view.clone();

        self.queue_on_main(move |view, mtm| {
            let panel = NSOpenPanel::openPanel(mtm);
            apply_common_dialog_options(&panel, &options);

            panel.setAllowsMultipleSelection(options.allow_multiple);
            panel.setCanChooseDirectories(options.pick_folders);
            panel.setCanChooseFiles(!options.pick_folders);

            if options.pick_folders {
                // Filters don't apply when selecting folders.
                panel.setAllowedContentTypes(&NSArray::new());
            } else if let Some(types) = options.allowed_types.as_ref() {
                // If any extension is "*" allow all types.
                let has_wildcard = types.iter().flat_map(|ft| ft.extensions.iter().copied()).any(|ext| ext == "*");

                if has_wildcard {
                    panel.setAllowedContentTypes(&NSArray::new());
                } else {
                    // Collect unique normalized extensions.
                    let mut extensions: Vec<&str> = types
                        .iter()
                        .flat_map(|ft| ft.extensions.iter().copied())
                        .map(|ext| ext.trim_start_matches('.'))
                        .filter(|ext| !ext.is_empty())
                        .collect();

                    extensions.sort_unstable();
                    extensions.dedup();

                    let empty_extensions = extensions.is_empty();

                    // Convert extensions to UTTypes.
                    let ut_types: Vec<Retained<UTType>> = extensions
                        .into_iter()
                        .filter_map(|ext| {
                            let ns_ext = NSString::from_str(ext);
                            UTType::typeWithFilenameExtension(&ns_ext)
                        })
                        .collect();

                    if !empty_extensions && ut_types.is_empty() {
                        warn!("No UTTypes resolved for extensions: allowing all file types");
                        panel.setAllowedContentTypes(&NSArray::new());
                    } else {
                        let arr = NSArray::from_retained_slice(&ut_types);
                        panel.setAllowedContentTypes(&arr);
                    }
                }
            } else {
                panel.setAllowedContentTypes(&NSArray::new());
            }

            let panel_cb = panel.clone();
            let ns_view_cb = ns_view.clone();

            let handler = RcBlock::new(move |resp| {
                // SAFETY: This runs on the main thread.
                let mtm = unsafe { MainThreadMarker::new_unchecked() };
                let view = ns_view_cb.get(mtm);

                let response = if resp == NSModalResponseOK {
                    let mut paths = Vec::new();
                    for url in panel_cb.URLs() {
                        if let Some(p) = util::nsurl_to_pathbuf(&url) {
                            paths.push(p);
                        }
                    }
                    FileDialogResponse::Opened(paths)
                } else {
                    FileDialogResponse::Cancelled
                };

                view.ivars().viewport.borrow_mut().file_dialog_event(view, node, response);
            });

            if let Some(window) = view.window() {
                panel.beginSheetModalForWindow_completionHandler(&window, &handler);
            } else {
                panel.beginWithCompletionHandler(&handler);
            }
        });
    }

    pub fn save_file_dialog(&self, node: Option<NodeId>, options: FileDialogOptions) {
        let Some(node) = node else {
            return;
        };
        let ns_view = self.ns_view.clone();

        self.queue_on_main(move |view, mtm| {
            let panel = NSSavePanel::savePanel(mtm);
            apply_common_dialog_options(&panel, &options);

            if let Some(label) = &options.filename_label {
                panel.setNameFieldLabel(Some(&NSString::from_str(label)));
            }

            if let Some(name) = &options.initial_name {
                panel.setNameFieldStringValue(&NSString::from_str(name));
            }

            let panel_cb = panel.clone();
            let ns_view_cb = ns_view.clone();

            let handler = RcBlock::new(move |resp| {
                // SAFETY: This runs on the main thread.
                let mtm = unsafe { MainThreadMarker::new_unchecked() };
                let view = ns_view_cb.get(mtm);

                let response = if resp == NSModalResponseOK {
                    panel_cb
                        .URL()
                        .and_then(|url| util::nsurl_to_pathbuf(&url))
                        .map(FileDialogResponse::Saved)
                        .unwrap_or(FileDialogResponse::Cancelled)
                } else {
                    FileDialogResponse::Cancelled
                };

                view.ivars().viewport.borrow_mut().file_dialog_event(view, node, response);
            });

            if let Some(window) = view.window() {
                panel.beginSheetModalForWindow_completionHandler(&window, &handler);
            } else {
                panel.beginWithCompletionHandler(&handler);
            }
        });
    }

    pub fn timer(&self, node: Option<NodeId>, delay: Duration) {
        let Some(node) = node else {
            return;
        };
        let ns_view = self.ns_view.clone();

        let delta_ns = delay.as_nanos().min(i64::MAX as u128) as i64;

        let when = DispatchTime::NOW.time(delta_ns);

        // Run on the main queue after the delay.
        let _ = DispatchQueue::main().after(when, move || {
            // SAFETY: We are on the main queue here.
            let mtm = unsafe { MainThreadMarker::new_unchecked() };
            let view = ns_view.get(mtm);
            view.ivars().viewport.borrow_mut().timer_event(node, view);
        });
    }

    pub fn alert<C>(&self, node: Option<NodeId>, png_bytes: Option<&'static [u8]>, title: &str, details: &str, options: &[(&'static str, C)])
    where
        C: Into<CommandId> + Copy,
    {
        let title = title.to_string();
        let details = details.to_string();
        let custom_options: Vec<(&'static str, CommandId)> = options.iter().map(|(label, cmd)| (*label, (*cmd).into())).collect();

        let ns_view = self.ns_view.clone();

        self.queue_on_main(move |view, mtm| {
            let alert = NSAlert::new(mtm);
            alert.setMessageText(&NSString::from_str(&title));
            alert.setInformativeText(&NSString::from_str(&details));

            if let Some(bytes) = png_bytes {
                // SAFETY: we know this pointer will be valid because it's &'static
                let data = unsafe { NSData::dataWithBytes_length(bytes.as_ptr().cast(), bytes.len()) };

                if let Some(img) = NSImage::initWithData(NSImage::alloc(), &data) {
                    unsafe { alert.setIcon(Some(&img)) }
                }
            }

            // Default Cocoa buttons, no event
            if custom_options.is_empty() {
                if let Some(window) = view.window() {
                    alert.beginSheetModalForWindow_completionHandler(&window, None);
                } else {
                    let _ = alert.runModal();
                }
                return;
            }

            for (label, _) in &custom_options {
                alert.addButtonWithTitle(&NSString::from_str(label));
            }

            let handle_response = move |view: &RosinView, resp: isize| {
                let Some(node) = node else { return };

                const NS_FIRST_BUTTON_RETURN: isize = 1000;
                let idx = resp.saturating_sub(NS_FIRST_BUTTON_RETURN) as usize;

                let Some((_, cmd)) = custom_options.get(idx) else {
                    return;
                };
                view.ivars().viewport.borrow_mut().command_event(view, Some(node), *cmd);
            };

            if let Some(window) = view.window() {
                let handler = RcBlock::new(move |resp: isize| {
                    // SAFETY: completion handler runs on main thread.
                    let mtm = unsafe { MainThreadMarker::new_unchecked() };
                    let view = ns_view.get(mtm);
                    handle_response(view, resp);
                });

                alert.beginSheetModalForWindow_completionHandler(&window, Some(&handler));
            } else {
                let resp = alert.runModal();
                handle_response(view, resp);
            }
        });
    }
}

fn apply_common_dialog_options(panel: &NSSavePanel, options: &FileDialogOptions) {
    panel.setShowsHiddenFiles(options.show_hidden);
    panel.setTreatsFilePackagesAsDirectories(options.browse_packages);
    panel.setCanCreateDirectories(options.allow_new_folders);

    if let Some(title) = &options.title {
        panel.setMessage(Some(&NSString::from_str(title)));
    }

    if let Some(prompt) = &options.submit_label {
        panel.setPrompt(Some(&NSString::from_str(prompt)));
    }

    if let Some(dir) = &options.initial_path {
        // If the caller accidentally passes a file path, fall back to its parent directory.
        let dir = if dir.is_dir() {
            dir.clone()
        } else {
            dir.parent().unwrap_or(dir).to_path_buf()
        };

        if let Some(url) = util::path_to_nsurl(&dir, true) {
            panel.setDirectoryURL(Some(&url));
        }
    }
}

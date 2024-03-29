#![forbid(unsafe_code)]

use std::{
    any::Any,
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use druid_shell::{
    kurbo, piet::Piet, Application, Cursor, FileDialogToken, FileInfo, IdleToken, KeyEvent, MouseButton, MouseButtons, MouseEvent, Region,
    Scale, TimerToken, WinHandler, WindowHandle,
};
use rosin_core::alloc::Alloc;

use crate::{libloader::LibLoader, prelude::*};

#[derive(Clone, Copy)]
pub struct WindowId(u32);

/// A description of a window.
pub struct WindowDesc<S: 'static, H: 'static> {
    pub(crate) view: ViewFn<S, H>,
    pub(crate) id: WindowId,
    pub(crate) title: Option<String>,
    pub(crate) size: (f32, f32),
    pub(crate) anim_tasks: Vec<Box<dyn AnimCallback<S>>>,
}

impl<S, H> WindowDesc<S, H> {
    pub fn new(view: ViewFn<S, H>) -> Self {
        Self {
            view,
            id: WindowId(0), // TODO - create a useful id
            title: None,
            size: (100.0, 100.0),
            anim_tasks: Vec::new(),
        }
    }

    pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = (width, height);
        self
    }

    pub fn add_anim_task(mut self, callback: impl Fn(&mut S, Duration) -> (Phase, ShouldStop) + 'static) -> Self {
        self.anim_tasks.push(Box::new(callback));
        self
    }

    pub fn get_id(&self) -> WindowId {
        // TODO
        self.id
    }
}

#[allow(dead_code)]
pub(crate) struct Window<S: 'static> {
    handle: WindowHandle,
    viewport: Viewport<S, WindowHandle>,
    viewfn: ViewFn<S, WindowHandle>,
    state: Rc<RefCell<S>>,
    libloader: Option<Arc<Mutex<LibLoader>>>,
    last_frame: Option<Instant>,
}

impl<S> Window<S> {
    pub fn new(
        resource_loader: ResourceLoader,
        viewfn: ViewFn<S, WindowHandle>,
        size: (f32, f32),
        state: Rc<RefCell<S>>,
        libloader: Option<Arc<Mutex<LibLoader>>>,
        anim_tasks: Vec<Box<dyn AnimCallback<S>>>,
    ) -> Self {
        let handle = WindowHandle::default();
        let mut rosin = if let Some(libloader) = libloader.clone() {
            let view_func = *libloader.lock().unwrap().get(viewfn.name).unwrap();
            let rosin = Viewport::new(resource_loader, view_func, size, handle.clone());
            let func: fn(Option<Rc<Alloc>>) = *libloader.lock().unwrap().get(b"set_thread_local_alloc").unwrap();
            func(Some(rosin.get_alloc()));
            rosin
        } else {
            Viewport::new(resource_loader, viewfn.func, size, handle.clone())
        };

        for anim in anim_tasks {
            rosin.add_anim_task(anim);
        }

        Self {
            handle,
            viewport: rosin,
            viewfn,
            state,
            libloader,
            last_frame: None,
        }
    }
}

impl<S> WinHandler for Window<S> {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
        self.viewport.set_handle(handle.clone());
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, piet: &mut Piet, _invalid: &Region) {
        // TODO - don't rebuild when not needed
        #[cfg(debug_assertions)]
        {
            self.viewport.update_phase(Phase::Build);
            self.handle.invalidate();
            self.handle.request_anim_frame();
        }

        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        if let Ok(libloader) = &mut self.libloader.as_ref().unwrap().try_lock() {
            // TODO - set a flag when poll returns true, and check that here
            if true {
                self.rosin.update_phase(Phase::Build);

                // Load the new view
                let view_callback = *libloader.get(self.view.name).unwrap();
                self.rosin.set_view(view_callback);

                // Share the local allocator
                let func: fn(Option<Rc<Alloc>>) = *libloader.get(b"set_thread_local_alloc").unwrap();
                func(Some(self.rosin.get_alloc()));
            }
        }

        let now = Instant::now();
        if let Some(last_frame) = self.last_frame {
            self.viewport
                .animation_frame(&mut self.state.borrow_mut(), now.duration_since(last_frame));
        }
        self.last_frame = Some(now);
        self.viewport.draw(&self.state.borrow(), Some(piet)).unwrap();

        if self.viewport.has_anim_tasks() {
            self.handle.request_anim_frame();
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn size(&mut self, size: kurbo::Size) {
        self.viewport.size((size.width as f32, size.height as f32))
    }

    fn scale(&mut self, scale: Scale) {
        self.viewport.scale((scale.x() as f32, scale.y() as f32));
    }

    fn rebuild_resources(&mut self) {}

    fn command(&mut self, _id: u32) {}

    fn save_as(&mut self, _token: FileDialogToken, _file: Option<FileInfo>) {}

    fn open_file(&mut self, _token: FileDialogToken, _file: Option<FileInfo>) {}

    fn key_down(&mut self, event: KeyEvent) -> bool {
        let mut state = self.state.borrow_mut();
        let result = self.viewport.key_event(&mut state, event);
        if !self.viewport.is_idle() {
            self.handle.invalidate();
            self.handle.request_anim_frame();
        }
        result
    }

    fn key_up(&mut self, event: KeyEvent) {
        let mut state = self.state.borrow_mut();
        self.viewport.key_event(&mut state, event);
        if !self.viewport.is_idle() {
            self.handle.invalidate();
            self.handle.request_anim_frame();
        }
    }

    fn wheel(&mut self, event: &MouseEvent) {
        let pointer_event = convert_event(event);
        self.viewport.pointer_wheel(&mut self.state.borrow_mut(), pointer_event);
        if !self.viewport.is_idle() {
            self.handle.invalidate();
            self.handle.request_anim_frame();
        }
    }

    fn zoom(&mut self, _delta: f64) {}

    fn mouse_move(&mut self, event: &MouseEvent) {
        let pointer_event = convert_event(event);
        self.handle.set_cursor(&Cursor::Arrow);
        self.viewport.pointer_move(&mut self.state.borrow_mut(), pointer_event);
        if !self.viewport.is_idle() {
            self.handle.invalidate();
            self.handle.request_anim_frame();
        }
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        let pointer_event = convert_event(event);
        let mut state = self.state.borrow_mut();
        self.viewport.pointer_down(&mut state, pointer_event);
        if !self.viewport.is_idle() {
            self.handle.invalidate();
            self.handle.request_anim_frame();
        }
    }

    fn mouse_up(&mut self, event: &MouseEvent) {
        let pointer_event = convert_event(event);
        let mut state = self.state.borrow_mut();
        self.viewport.pointer_up(&mut state, pointer_event);
        if !self.viewport.is_idle() {
            self.handle.invalidate();
            self.handle.request_anim_frame();
        }
    }

    fn mouse_leave(&mut self) {
        let mut state = self.state.borrow_mut();
        self.viewport.pointer_leave(&mut state);
        if !self.viewport.is_idle() {
            self.handle.invalidate();
            self.handle.request_anim_frame();
        }
    }

    fn timer(&mut self, _token: TimerToken) {}

    fn got_focus(&mut self) {
        let mut state = self.state.borrow_mut();
        self.viewport.got_focus(&mut state);
        if !self.viewport.is_idle() {
            self.handle.invalidate();
            self.handle.request_anim_frame();
        }
    }

    fn lost_focus(&mut self) {
        let mut state = self.state.borrow_mut();
        self.viewport.lost_focus(&mut state);
        if !self.viewport.is_idle() {
            self.handle.invalidate();
            self.handle.request_anim_frame();
        }
    }

    fn request_close(&mut self) {
        let mut state = self.state.borrow_mut();
        self.viewport.close(&mut state);
        self.handle.close();
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn idle(&mut self, _token: IdleToken) {}
}

fn convert_event(event: &MouseEvent) -> RawPointerEvent {
    RawPointerEvent {
        window_pos_x: event.pos.x,
        window_pos_y: event.pos.y,
        wheel_x: event.wheel_delta.x,
        wheel_y: event.wheel_delta.y,
        button: convert_button(event.button),
        buttons: convert_buttons(event.buttons),
        mods: convert_mods(event.mods),
        count: event.count,
        focus: event.focus,
    }
}

fn convert_button(button: MouseButton) -> PointerButton {
    if button.is_left() {
        PointerButton::Left
    } else if button.is_right() {
        PointerButton::Right
    } else if button.is_middle() {
        PointerButton::Middle
    } else if button.is_x1() {
        PointerButton::X1
    } else {
        PointerButton::X2
    }
}

fn convert_buttons(buttons: MouseButtons) -> PointerButtons {
    let mut result = PointerButtons::new();

    if buttons.has_left() {
        result.insert(PointerButton::Left);
    }
    if buttons.has_right() {
        result.insert(PointerButton::Right);
    }
    if buttons.has_middle() {
        result.insert(PointerButton::Middle);
    }
    if buttons.has_x1() {
        result.insert(PointerButton::X1);
    }
    if buttons.has_x2() {
        result.insert(PointerButton::X2);
    }

    result
}

fn convert_mods(mods: druid_shell::Modifiers) -> Modifiers {
    Modifiers::from_bits_truncate(mods.raw().bits())
}

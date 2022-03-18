#![forbid(unsafe_code)]

use std::{
    any::Any,
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex}, time::Duration,
};

use druid_shell::{
    kurbo, piet::Piet, Application, Cursor, FileDialogToken, FileInfo, IdleToken, KeyEvent, MouseEvent, Region, Scale, TimerToken,
    WinHandler, WindowHandle,
};
use rosin_core::alloc::Alloc;

use crate::{libloader::LibLoader, prelude::*};

#[derive(Clone, Copy)]
pub struct WindowId(u32);

/// A description of a window.
pub struct WindowDesc<S: 'static, H: 'static> {
    pub(crate) view: View<S, H>,
    pub(crate) id: WindowId,
    pub(crate) title: Option<String>,
    pub(crate) size: (f32, f32),
    pub(crate) anim_tasks: Vec<Box<dyn AnimCallback<S>>>,
}

impl<S, H> WindowDesc<S, H> {
    pub fn new(view: View<S, H>) -> Self {
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

    pub fn add_anim_task(&mut self, callback: impl Fn(&mut S, Duration) -> (Phase, ShouldStop) + 'static) {
        self.anim_tasks.push(Box::new(callback));
    }

    pub fn get_id(&self) -> WindowId {
        // TODO
        self.id
    }
}

#[allow(dead_code)]
pub(crate) struct Window<S: 'static> {
    handle: WindowHandle,
    rosin: RosinWindow<S, WindowHandle>,
    view: View<S, WindowHandle>,
    state: Rc<RefCell<S>>,
    libloader: Option<Arc<Mutex<LibLoader>>>,
    last_ext: u32,
}

impl<S> Window<S> {
    pub fn new(
        resource_loader: Arc<Mutex<ResourceLoader>>,
        view: View<S, WindowHandle>,
        size: (f32, f32),
        state: Rc<RefCell<S>>,
        libloader: Option<Arc<Mutex<LibLoader>>>,
        anim_tasks: Vec<Box<dyn AnimCallback<S>>>,
    ) -> Self {
        let mut rosin = if let Some(libloader) = libloader.clone() {
            let view_func = *libloader.lock().unwrap().get(view.name).unwrap();
            let rosin = RosinWindow::new(resource_loader, view_func, size);
            let func: fn(Option<Rc<Alloc>>) = *libloader.lock().unwrap().get(b"set_thread_local_alloc").unwrap();
            func(Some(rosin.get_alloc()));
            rosin
        } else {
            RosinWindow::new(resource_loader, view.func, size)
        };

        for anim in anim_tasks {
            rosin.add_anim_task(anim);
        }

        Self {
            handle: WindowHandle::default(),
            rosin,
            view,
            state,
            libloader,
            last_ext: 0,
        }
    }
}

impl<S> WinHandler for Window<S> {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
        self.rosin.set_handle(handle.clone());
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, piet: &mut Piet, _invalid: &Region) {
        // TODO - don't rebuild when not needed
        #[cfg(debug_assertions)]
        {
            #[cfg(feature = "hot-reload")]
            if let Ok(libloader) = self.libloader.as_ref().unwrap().try_lock() {
                if self.last_ext < libloader.get_ext() {
                    let view_callback = *libloader.get(self.view.name).unwrap();
                    self.rosin.set_view(view_callback);

                    let func: fn(Option<Rc<Alloc>>) = *libloader.get(b"set_thread_local_alloc").unwrap();
                    func(Some(self.rosin.get_alloc()));

                    self.handle.invalidate();
                    self.handle.request_anim_frame();
                }
            }

            self.rosin.update_phase(Phase::Build);
            self.handle.invalidate();
            self.handle.request_anim_frame();
        }

        self.rosin.draw(&mut self.state.borrow_mut(), piet).unwrap();

        if self.rosin.has_anim_tasks() {
            self.handle.request_anim_frame();
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn size(&mut self, size: kurbo::Size) {
        self.rosin.size((size.width as f32, size.height as f32))
    }

    fn scale(&mut self, scale: Scale) {
        self.rosin.scale((scale.x() as f32, scale.y() as f32));
    }

    fn rebuild_resources(&mut self) {}

    fn command(&mut self, id: u32) {}

    fn save_as(&mut self, token: FileDialogToken, file: Option<FileInfo>) {}

    fn open_file(&mut self, token: FileDialogToken, file: Option<FileInfo>) {}

    fn key_down(&mut self, event: KeyEvent) -> bool {
        let mut state = self.state.borrow_mut();
        self.rosin.key_down(&mut state, event)
    }

    fn key_up(&mut self, event: KeyEvent) {}

    fn wheel(&mut self, event: &MouseEvent) {}

    fn zoom(&mut self, delta: f64) {}

    fn mouse_move(&mut self, event: &MouseEvent) {
        self.handle.set_cursor(&Cursor::Arrow);
        self.rosin.mouse_move(&mut self.state.borrow_mut(), event);
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        let mut state = self.state.borrow_mut();
        self.rosin.mouse_down(&mut state, event);
        if !self.rosin.is_idle() {
            self.handle.invalidate();
            self.handle.request_anim_frame();
        }
    }

    fn mouse_up(&mut self, event: &MouseEvent) {}

    fn mouse_leave(&mut self) {}

    fn timer(&mut self, token: TimerToken) {}

    fn got_focus(&mut self) {}

    fn lost_focus(&mut self) {}

    fn request_close(&mut self) {
        self.handle.close();
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn idle(&mut self, _token: IdleToken) {}
}

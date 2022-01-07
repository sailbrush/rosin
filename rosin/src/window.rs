#![forbid(unsafe_code)]

use std::{
    any::Any,
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use druid_shell::{
    kurbo, piet::Piet, Application, FileDialogToken, FileInfo, IdleToken, KeyEvent, MouseEvent, Region, Scale, TimerToken, WinHandler,
    WindowHandle,
};
use rosin_core::alloc::Alloc;

use crate::{libloader::LibLoader, prelude::*};

#[derive(Clone, Copy)]
pub struct WindowId(u32);

/// A description of a window.
pub struct WindowDesc<T: 'static> {
    pub(crate) view: View<T>,
    pub(crate) id: WindowId,
    pub(crate) title: Option<String>,
    pub(crate) size: (f32, f32),
}

impl<T> WindowDesc<T> {
    pub fn new(view: View<T>) -> Self {
        Self {
            view,
            id: WindowId(0),
            title: None,
            size: (100.0, 100.0),
        }
    }

    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = (width, height);
        self
    }

    pub fn get_id(&self) -> WindowId {
        self.id
    }
}

#[allow(dead_code)]
pub(crate) struct Window<T: 'static> {
    rosin: RosinWindow<T, WindowHandle>,
    view: View<T>,
    state: Rc<RefCell<T>>,
    libloader: Option<Arc<Mutex<LibLoader>>>,
    last_ext: u32,
}

impl<T> Window<T> {
    pub fn new(
        sheet_loader: Arc<Mutex<SheetLoader>>,
        view: View<T>,
        size: (f32, f32),
        state: Rc<RefCell<T>>,
        libloader: Option<Arc<Mutex<LibLoader>>>,
    ) -> Self {
        let view_callback = if let Some(libloader) = libloader.clone() {
            *libloader.lock().unwrap().get(view.name).unwrap()
        } else {
            view.func
        };

        let rosin = RosinWindow::new(sheet_loader, view_callback, size);

        if let Some(libloader) = libloader.clone() {
            let func: fn(Option<Rc<Alloc>>) = *libloader.lock().unwrap().get(b"set_thread_local_alloc").unwrap();
            func(Some(rosin.get_alloc()));
        }

        Self {
            rosin,
            view,
            state,
            libloader,
            last_ext: 0,
        }
    }
}

impl<T> WinHandler for Window<T> {
    fn connect(&mut self, handle: &WindowHandle) {
        self.rosin.set_handle(handle.clone())
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, piet: &mut Piet, _invalid: &Region) {
        self.rosin.draw(&self.state.borrow(), piet).unwrap();
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn size(&mut self, size: kurbo::Size) {
        self.rosin.size((size.width as f32, size.height as f32))
    }

    fn scale(&mut self, scale: Scale) {}

    fn rebuild_resources(&mut self) {}

    fn command(&mut self, id: u32) {}

    fn save_as(&mut self, token: FileDialogToken, file: Option<FileInfo>) {}

    fn open_file(&mut self, token: FileDialogToken, file: Option<FileInfo>) {}

    fn key_down(&mut self, event: KeyEvent) -> bool {
        false
    }

    fn key_up(&mut self, event: KeyEvent) {}

    fn wheel(&mut self, event: &MouseEvent) {}

    fn zoom(&mut self, delta: f64) {}

    fn mouse_move(&mut self, event: &MouseEvent) {}

    fn mouse_down(&mut self, event: &MouseEvent) {
        let mut ctx = EventCtx {};
        self.rosin
            .click(&mut self.state.borrow_mut(), &mut ctx, (event.pos.x as f32, event.pos.y as f32));
        if !self.rosin.is_idle() {
            self.rosin.get_handle_ref().unwrap().request_anim_frame();
        }
    }

    fn mouse_up(&mut self, event: &MouseEvent) {}

    fn mouse_leave(&mut self) {}

    fn timer(&mut self, token: TimerToken) {}

    fn got_focus(&mut self) {}

    fn lost_focus(&mut self) {}

    fn request_close(&mut self) {
        self.rosin.get_handle_ref().unwrap().close();
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn idle(&mut self, _token: IdleToken) {
        #[cfg(debug_assertions)]
        {
            #[cfg(feature = "hot-reload")]
            if let Ok(libloader) = self.libloader.as_ref().unwrap().try_lock() {
                if self.last_ext < libloader.get_ext() {
                    let view_callback = *libloader.get(self.view.name).unwrap();
                    self.rosin.set_view(view_callback);

                    let func: fn(Option<Rc<Alloc>>) = *libloader.get(b"set_thread_local_alloc").unwrap();
                    func(Some(self.rosin.get_alloc()));

                    self.rosin.get_handle_mut().unwrap().request_anim_frame();
                }
            }

            self.rosin.update_phase(Phase::Build);
            self.rosin.get_handle_mut().unwrap().request_anim_frame();

            let mut idle_handle = self.rosin.get_handle_mut().unwrap().get_idle_handle().unwrap();
            idle_handle.schedule_idle(IdleToken::new(0));
        }
    }
}

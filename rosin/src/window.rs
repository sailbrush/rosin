#![forbid(unsafe_code)]

use std::{any::Any, cell::RefCell, rc::Rc, sync::{Arc, Mutex}};

use druid_shell::{
    kurbo, piet::Piet, Application, FileDialogToken, FileInfo, IdleToken, KeyEvent, MouseEvent, Region, Scale, TimerToken, WinHandler,
    WindowHandle,
};

use crate::{prelude::*, libloader::LibLoader};

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
    libloader: Arc<Mutex<LibLoader>>,
}

impl<T> Window<T> {
    pub fn new(sheet_loader: Arc<SheetLoader>, libloader: Arc<Mutex<LibLoader>>, view: View<T>, size: (f32, f32), state: Rc<RefCell<T>>) -> Self {
        let view_callback = view.get(&libloader.lock().unwrap());
        Self {
            rosin: RosinWindow::new(sheet_loader, view_callback, size),
            view,
            state,
            libloader,
        }
    }
}

impl<T> WinHandler for Window<T> {
    fn connect(&mut self, handle: &WindowHandle) {
        self.rosin.set_handle(handle.clone())
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, piet: &mut Piet, _invalid: &Region) {
        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        {
            let mut libloader = self.libloader.lock().unwrap();
            if libloader.poll().unwrap() {
                let view_callback = self.view.get(&libloader);
                self.rosin.set_view(view_callback);
            }
        }

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

    fn idle(&mut self, token: IdleToken) {}
}

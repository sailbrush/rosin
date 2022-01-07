#![forbid(unsafe_code)]
#![allow(unused_imports)]

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{libloader::*, prelude::*, window::Window};

use druid_shell::{Application, IdleToken, WindowBuilder};
use rosin_core::grc::Registry;
use rosin_core::prelude::*;

pub struct AppLauncher<T: 'static> {
    sheet_loader: Arc<SheetLoader>,
    windows: Vec<WindowDesc<T>>,
}

impl<T> AppLauncher<T> {
    pub fn new(sheet_loader: SheetLoader, window: WindowDesc<T>) -> Self {
        Self {
            sheet_loader: Arc::new(sheet_loader),
            windows: vec![window],
        }
    }

    pub fn add_window(mut self, window: WindowDesc<T>) -> Self {
        self.windows.push(window);
        self
    }

    pub fn run(self, state: T) -> Result<(), Box<dyn std::error::Error>> {
        let state = Rc::new(RefCell::new(state));

        // Set up libloader
        #[cfg(not(all(debug_assertions, feature = "hot-reload")))]
        let libloader: Option<Arc<Mutex<LibLoader>>> = None;

        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        let libloader = {
            // Use the name of the current binary to find the library
            let cmd = std::env::args().next().unwrap();
            let cmd_path = std::path::Path::new(&cmd);
            let lib_name = cmd_path.with_file_name(format!(
                "lib{}",
                cmd_path.with_extension(DYLIB_EXT).file_name().unwrap().to_str().unwrap()
            ));
            let lib_path = std::env::current_dir().unwrap().join(&lib_name);
            let loader = Arc::new(Mutex::new(LibLoader::new(lib_path).expect("[Rosin] Hot-reload: Failed to init")));

            // Init Grc registry
            if let Ok(mut loader) = loader.try_lock() {
                if let Ok(_) = loader.poll() {
                    let func: fn(Arc<Mutex<Registry>>) -> Result<(), Arc<Mutex<Registry>>> = *loader.get(b"set_grc_registry").unwrap();
                    func(Registry::get_grc_registry().clone()).expect("Failed to set grc registry");
                }
            }

            // Start a thread that periodically polls the libloader
            let thread_loader = loader.clone();
            thread::spawn(move || loop {
                if let Ok(mut loader) = thread_loader.try_lock() {
                    if let Ok(true) = loader.poll() {
                        let func: fn(Arc<Mutex<Registry>>) -> Result<(), Arc<Mutex<Registry>>> = *loader.get(b"set_grc_registry").unwrap();
                        func(Registry::get_grc_registry().clone()).expect("Failed to set grc registry");
                    }
                }
                thread::sleep(Duration::from_millis(100));
            });

            Some(loader)
        };

        // Create Druid Applicaiton
        let druid_app = Application::new().unwrap();

        for desc in self.windows {
            let mut builder = WindowBuilder::new(druid_app.clone());

            let handler = Window::new(self.sheet_loader.clone(), desc.view, desc.size, state.clone(), libloader.clone());
            builder.set_handler(Box::new(handler));

            if let Some(title) = desc.title {
                builder.set_title(title);
            }

            builder.set_size((desc.size.0 as f64, desc.size.1 as f64).into());

            let window = builder.build().unwrap();

            #[cfg(all(debug_assertions, feature = "hot-reload"))]
            {
                let mut idle_handle = window.get_idle_handle().unwrap();
                idle_handle.schedule_idle(IdleToken::new(0));
            }

            window.show();
        }

        // Run the app
        druid_app.run(None);

        Ok(())
    }
}

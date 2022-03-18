#![forbid(unsafe_code)]

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{libloader::*, prelude::*, window::Window};

use druid_shell::{Application, WindowBuilder};
use rosin_core::prelude::*;

pub struct AppLauncher<S: 'static> {
    resource_loader: Arc<Mutex<ResourceLoader>>,
    windows: Vec<WindowDesc<S, WindowHandle>>,
}

impl<S> AppLauncher<S> {
    pub fn new(resource_loader: ResourceLoader, window: WindowDesc<S, WindowHandle>) -> Self {
        Self {
            resource_loader: Arc::new(Mutex::new(resource_loader)),
            windows: vec![window],
        }
    }

    pub fn add_window(mut self, window: WindowDesc<S, WindowHandle>) -> Self {
        self.windows.push(window);
        self
    }

    pub fn run(self, state: S) -> Result<(), Box<dyn std::error::Error>> {
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

            // Start a thread that periodically polls the libloader
            let thread_loader = loader.clone();
            thread::spawn(move || loop {
                if let Ok(mut loader) = thread_loader.try_lock() {
                    loader.poll().unwrap();
                }
                thread::sleep(Duration::from_millis(100));
            });

            Some(loader)
        };

        let thread_resource_loader = self.resource_loader.clone();
        #[cfg(debug_assertions)]
        {
            thread::spawn(move || loop {
                thread_resource_loader.lock().unwrap().poll().unwrap();
                thread::sleep(Duration::from_millis(100));
            });
        }

        // Create Druid Applicaiton
        let druid_app = Application::new().unwrap();

        for desc in self.windows {
            let mut builder = WindowBuilder::new(druid_app.clone());

            let handler = Window::new(self.resource_loader.clone(), desc.view, desc.size, state.clone(), libloader.clone());
            builder.set_handler(Box::new(handler));

            if let Some(title) = desc.title {
                builder.set_title(title);
            }

            builder.set_size((desc.size.0 as f64, desc.size.1 as f64).into());

            let window = builder.build().unwrap();

            window.show();
        }

        // Run the app
        druid_app.run(None);

        Ok(())
    }
}

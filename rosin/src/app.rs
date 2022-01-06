#![forbid(unsafe_code)]

use std::{cell::RefCell, rc::Rc, sync::{Arc, Mutex}};

use crate::{libloader::*, prelude::*, window::Window};

use druid_shell::{Application, WindowBuilder};
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
        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        let libloader = {
            // TODO - can probably set an env variable in a build script or something
            // Use the name of the current binary to find the library
            let cmd = std::env::args().next().unwrap();
            let cmd_path = std::path::Path::new(&cmd);
            let lib_name = cmd_path.with_file_name(format!(
                "_{}",
                cmd_path.with_extension(DYLIB_EXT).file_name().unwrap().to_str().unwrap()
            ));
            let lib_path = std::env::current_dir().unwrap().join(&lib_name);
            LibLoader::new(lib_path).expect("[Rosin] Hot-reload: Failed to init")
        };

        #[cfg(not(all(debug_assertions, feature = "hot-reload")))]
        let libloader = LibLoader {};

        let libloader = Arc::new(Mutex::new(libloader));

        // Create Druid Applicaiton
        let druid_app = Application::new().unwrap();

        for desc in self.windows {
            let mut builder = WindowBuilder::new(druid_app.clone());

            let handler = Window::new(self.sheet_loader.clone(), libloader.clone(), desc.view, desc.size, state.clone());
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

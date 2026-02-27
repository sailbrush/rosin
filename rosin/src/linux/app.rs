
use wayland_client::Connection;
use std::sync::OnceLock;
use std::rc::Rc;
use std::cell::RefCell;
use crate::linux::wayland_state::WaylandState;
use crate::prelude::*;
use crate::linux::wayland_state::*;

static _APP_STARTED: OnceLock<()> = OnceLock::new();

pub(crate) struct AppLauncher<S: Sync + 'static> {
    windows: Vec<WindowDesc<S>>,
    _translation_map: Option<TranslationMap>,
    wgpu_config: WgpuConfig,
    _state: Option<Rc<RefCell<S>>>,

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    hot_reloader: RefCell<Option<crate::mac::hot::HotReloader>>,
}

impl<S: Sync + 'static> AppLauncher<S> {
    pub fn new(window: WindowDesc<S>) -> Self {
        Self {
            windows: vec![window],
            _translation_map: None,
            wgpu_config: WgpuConfig::default(),
            _state: None,
    
            #[cfg(all(feature = "hot-reload", debug_assertions))]
            hot_reloader: RefCell::new(None),
        }
    }

    pub fn with_wgpu_config(mut self, config: WgpuConfig) -> Self {
        self.wgpu_config = config;
        self
    }

    pub fn add_window(mut self, window: WindowDesc<S>) -> Self {
        self.windows.push(window);
        self
    }

    // No hot-reload, no serde requirement
    #[cfg(not(all(feature = "hot-reload", debug_assertions)))]
    pub fn run(self, _state: S, _translation_map: TranslationMap) -> Result<(), LaunchError> {
        

        let conn = Connection::connect_to_env().unwrap();

        let mut event_queue = conn.new_event_queue();
        let qhandle = event_queue.handle();

        let display = conn.display();
        display.get_registry(&qhandle, ());
        let win_desc = window_desc_to_wayland(self.windows[0].clone());

        let mut state = WaylandState {
            running: true,
            base_surface: None,
            buffer: None,
            wm_base: None,
            xdg_surface: None,
            xdg_decorations: None,
            configured: false,
            window_desc: win_desc
        };

        println!("Starting the example window app, press <ESC> to quit.");

        while state.running {
            event_queue.blocking_dispatch(&mut state).unwrap();
        }
        Ok(())
    }

    // Yes hot-reload, yes serde requirement
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn run(mut self, mut _state: S, _translation_map: TranslationMap) -> Result<(), LaunchError>
    where
        S: serde::Serialize + serde::de::DeserializeOwned + crate::typehash::TypeHash + 'static,
    {
        // TODO
        Ok(())
    }
}

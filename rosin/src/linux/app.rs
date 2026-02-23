use std::sync::OnceLock;
use std::{cell::RefCell, rc::Rc};

use crate::prelude::*;

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
        // TODO
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

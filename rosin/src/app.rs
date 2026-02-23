//! Types for configuring and launching an application instance.

use crate::prelude::*;

/// An error returned when launching the application fails.
#[derive(Debug, Copy, Clone)]
pub enum LaunchError {
    /// The application has already been launched.
    AlreadyStarted,
    /// The application wasn't launched on the main thread.
    NotOnMainThread,
}

/// The primary entry point for configuring and launching a Rosin application.
pub struct AppLauncher<S: Sync + 'static>(pub(crate) crate::platform::app::AppLauncher<S>);

impl<S: Sync + 'static> AppLauncher<S> {
    /// Creates a new [`AppLauncher`] initialized with at least one window.
    ///
    /// The application will not start until [`run`](Self::run) is called.
    pub fn new(window: WindowDesc<S>) -> Self {
        Self(crate::platform::app::AppLauncher::new(window))
    }

    /// Queue additional windows to be created when the application is run.
    pub fn add_window(self, window: WindowDesc<S>) -> Self {
        Self(self.0.add_window(window))
    }

    /// Overrides the default `wgpu` rendering configuration.
    ///
    /// Use this to request specific hardware features or to prioritize battery life
    /// over rendering performance.
    pub fn with_wgpu_config(self, config: WgpuConfig) -> Self {
        Self(self.0.with_wgpu_config(config))
    }

    /// Runs the application event loop. This can only be called once per process.
    ///
    /// This call blocks the current thread while the application is running.
    /// The event loop stops when all windows have closed or when [`WindowHandle::request_exit`] is called.
    #[cfg(not(all(feature = "hot-reload", debug_assertions)))]
    pub fn run(self, state: S, translations: TranslationMap) -> Result<(), LaunchError> {
        self.0.run(state, translations)
    }

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn run(self, state: S, translations: TranslationMap) -> Result<(), LaunchError>
    where
        S: serde::Serialize + serde::de::DeserializeOwned + crate::typehash::TypeHash,
    {
        self.0.run(state, translations)
    }
}

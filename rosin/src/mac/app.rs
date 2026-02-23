use std::{cell::RefCell, rc::Rc, thread};
use std::{panic, sync::OnceLock};

use objc2::{DeclaredClass, MainThreadOnly, define_class, msg_send, rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::{NSApp, NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate};
use objc2_foundation::{MainThreadMarker, NSNotification, NSObject, NSObjectProtocol};
use pollster::FutureExt;

use crate::{
    log::error,
    mac::{util, window},
    prelude::*,
    vello::{self, AaSupport},
    wgpu::{self, ExperimentalFeatures},
};

static APP_STARTED: OnceLock<()> = OnceLock::new();

pub(crate) struct AppLauncher<S: Sync + 'static> {
    windows: Vec<WindowDesc<S>>,
    translation_map: Option<TranslationMap>,
    wgpu_config: WgpuConfig,
    state: Option<Rc<RefCell<S>>>,

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    hot_reloader: RefCell<Option<crate::mac::hot::HotReloader>>,
}

impl<S: Sync + 'static> AppLauncher<S> {
    pub fn new(window: WindowDesc<S>) -> Self {
        Self {
            windows: vec![window],
            translation_map: None,
            wgpu_config: WgpuConfig::default(),
            state: None,

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
    pub fn run(self, state: S, translation_map: TranslationMap) -> Result<(), LaunchError> {
        self.run_impl(state, translation_map, |s| Box::new(s) as Box<dyn AppDelegateTrait>)
    }

    // Yes hot-reload, yes serde requirement
    #[cfg(all(feature = "hot-reload", debug_assertions))]
    pub fn run(mut self, mut state: S, translation_map: TranslationMap) -> Result<(), LaunchError>
    where
        S: serde::Serialize + serde::de::DeserializeOwned + crate::typehash::TypeHash + 'static,
    {
        use std::{env, fs};

        if let Some(loader) = crate::mac::hot::HotReloader::new() {
            'hot_reload: {
                if let Ok(snapshot_path) = env::var("ROSIN_HOT_RELOAD_SNAPSHOT") {
                    let snapshot_json = match fs::read_to_string(&snapshot_path) {
                        Ok(s) => s,
                        Err(e) => {
                            error!("Hot-reload failed to read snapshot file: {e}");
                            break 'hot_reload;
                        }
                    };

                    let snapshot: crate::mac::hot::HotReloadSnapshot<S> =
                        match crate::reactive::serde_impl::serde_scope(|| serde_json::from_str(&snapshot_json)) {
                            Ok(s) => s,
                            Err(e) => {
                                error!("Hot-reload failed to parse snapshot JSON: {e}");
                                break 'hot_reload;
                            }
                        };

                    state = snapshot.state;

                    self.windows = snapshot.windows.into_iter().map(|w| w.convert::<S>(&loader.lib)).collect();
                }
            }

            *self.hot_reloader.borrow_mut() = Some(loader);
        } else {
            error!("Hot-reload failed to init.");
        }

        self.run_impl(state, translation_map, |s| Box::new(s) as Box<dyn AppDelegateTrait>)
    }

    fn run_impl<F>(mut self, state: S, translation_map: TranslationMap, box_data: F) -> Result<(), LaunchError>
    where
        F: FnOnce(AppLauncher<S>) -> Box<dyn AppDelegateTrait>,
    {
        if APP_STARTED.set(()).is_err() {
            return Err(LaunchError::AlreadyStarted);
        }

        // Start loading fonts in a background thread to reduce time to first frame.
        let _ = thread::spawn(|| {
            if let Err(e) = panic::catch_unwind(global_font_ctx) {
                error!("Font loading thread panicked: {:?}", e);
            }
        });

        let mtm = MainThreadMarker::new().ok_or(LaunchError::NotOnMainThread)?;
        let ns_app = NSApplication::sharedApplication(mtm);

        self.state = Some(Rc::new(RefCell::new(state)));
        self.translation_map = Some(translation_map);

        let data = box_data(self);
        let ns_app_delegate: Retained<RosinAppDelegate> = unsafe {
            let delegate = mtm.alloc().set_ivars(AppDelegateIvars { data });
            msg_send![super(delegate), init]
        };
        ns_app.setDelegate(Some(ProtocolObject::from_ref(&*ns_app_delegate)));
        ns_app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

        ns_app.run();

        Ok(())
    }

    fn application_did_finish_launching_impl(&self, _delegate: &RosinAppDelegate) {
        let Some(mtm) = MainThreadMarker::new() else {
            // app kit should always call this on the main thread
            return;
        };

        NSApp(mtm).activate();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: self.wgpu_config.backends,
            ..Default::default()
        });
        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: self.wgpu_config.power_preference,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .block_on()
        {
            Ok(adapter) => adapter,
            Err(e) => util::fatal_alert_and_quit(mtm, "GPU initialization failed", &format!("Failed to request a WGPU adapter.\n\n{e}")),
        };

        let (device, queue) = match adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("RosinDevice"),
                required_features: self.wgpu_config.features,
                required_limits: self.wgpu_config.limits.clone(),
                memory_hints: self.wgpu_config.memory_hints.clone(),
                trace: wgpu::Trace::Off,
                experimental_features: ExperimentalFeatures::disabled(),
            })
            .block_on()
        {
            Ok((device, queue)) => (device, queue),
            Err(e) => util::fatal_alert_and_quit(mtm, "GPU initialization failed", &format!("Failed to create a WGPU device.\n\n{e}")),
        };

        let compositor = Compositor {
            blitter: RefCell::new(None),
            custom: RefCell::new(None),
        };

        let vello_renderer = {
            let renderer = match vello::Renderer::new(
                &device,
                vello::RendererOptions {
                    use_cpu: false,
                    antialiasing_support: AaSupport::all(),
                    num_init_threads: None,
                    pipeline_cache: None,
                },
            ) {
                Ok(r) => r,
                Err(e) => util::fatal_alert_and_quit(mtm, "Renderer initialization failed", &format!("Failed to create the Vello renderer.\n\n{e}")),
            };

            Rc::new(RefCell::new(renderer))
        };

        let gpu_ctx = Rc::new(GpuCtx {
            instance,
            adapter,
            device,
            queue,
            compositor,
        });

        let state = self.state.clone().unwrap(); // Unwrap ok: state be Some() to launch the app.
        let translation_map = self.translation_map.clone().unwrap_or_default();

        for desc in &self.windows {
            window::create_window(mtm, desc, state.clone(), translation_map.clone(), gpu_ctx.clone(), vello_renderer.clone());
        }

        #[cfg(all(feature = "hot-reload", debug_assertions))]
        {
            // Don't start the timer if creating the reloader failed.
            if self.hot_reloader.borrow().is_none() {
                return;
            }

            unsafe {
                objc2_foundation::NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                    1.0 / 5.0,
                    _delegate,
                    objc2::sel!(hot_reload_tick),
                    None,
                    true,
                )
            };
        }
    }
}

pub(crate) trait AppDelegateTrait {
    fn application_did_finish_launching(&self, delegate: &RosinAppDelegate);
    fn terminate(&self) {}

    #[cfg(all(feature = "hot-reload", debug_assertions))]
    fn hot_reload_tick(&self);
}

// No hot-reload, no serde requirement
#[cfg(not(all(feature = "hot-reload", debug_assertions)))]
impl<S: Sync + 'static> AppDelegateTrait for AppLauncher<S> {
    fn application_did_finish_launching(&self, delegate: &RosinAppDelegate) {
        self.application_did_finish_launching_impl(delegate);
    }
}

// Yes hot-reload, yes serde requirement
#[cfg(all(feature = "hot-reload", debug_assertions))]
impl<S> AppDelegateTrait for AppLauncher<S>
where
    S: Sync + serde::Serialize + crate::typehash::TypeHash + 'static,
{
    fn application_did_finish_launching(&self, delegate: &RosinAppDelegate) {
        self.application_did_finish_launching_impl(delegate);
    }

    fn hot_reload_tick(&self) {
        let Some(state_rc) = self.state.as_ref() else {
            return;
        };
        if let Some(hot_reloader) = &mut *self.hot_reloader.borrow_mut() {
            hot_reloader.reload_if_changed(state_rc);
        }
    }

    fn terminate(&self) {
        *self.hot_reloader.borrow_mut() = None;
    }
}

/// A type-erased wrapper for the app delegate data, since objc isn't compatible with Rust generics.
pub(crate) struct AppDelegateIvars {
    data: Box<dyn AppDelegateTrait>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = AppDelegateIvars]
    pub(crate) struct RosinAppDelegate;

    unsafe impl NSObjectProtocol for RosinAppDelegate {}

    impl RosinAppDelegate {
        #[cfg(all(feature = "hot-reload", debug_assertions))]
        #[unsafe(method(hot_reload_tick))]
        fn __hot_reload_tick(&self) {
            self.ivars().data.hot_reload_tick();
        }
    }

    unsafe impl NSApplicationDelegate for RosinAppDelegate {
        /// Returns a Boolean value that indicates if the app terminates once the last window closes.
        ///
        /// Parameters:
        /// * `sender` – The application object whose last window was closed.
        ///
        /// Return Value: `false` if the application should not be terminated when its last window is closed;
        /// otherwise, `true` to terminate the application.
        ///
        /// The application sends this message to your delegate when the application's last window is closed. It
        /// sends this message regardless of whether there are still panels open. (A panel in this case is defined
        /// as being an instance of `NSPanel` or one of its subclasses.)
        ///
        /// If your implementation returns `false`, control returns to the main event loop and the application is
        /// not terminated. If you return `true`, your delegate's `applicationShouldTerminate(_:)` method is
        /// subsequently invoked to confirm that the application should be terminated.
        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn __application_should_terminate_after_last_window_closed(&self, _sender: &NSApplication) -> bool {
            if let Some(mtm) = MainThreadMarker::new() {
                NSApp(mtm).stop(None);
            }
            self.ivars().data.terminate();
            false
        }

        /// Tells the delegate that the app's initialization is complete but it hasn't received its first event.
        ///
        /// Parameters:
        /// * `notification` – A notification named `didFinishLaunchingNotification`. Calling the `object` method of
        ///   this notification returns the `NSApplication` object itself.
        ///
        /// Delegates can implement this method to perform further initialization. This method is called after the
        /// application's main run loop has been started but before it has processed any events. If the application
        /// was launched by the user opening a file, the delegate's `application(_:openFile:)` method is called
        /// before this method. If you want to perform initialization before any files are opened, implement the
        /// `applicationWillFinishLaunching(_:)` method in your delegate, which is called before
        /// `application(_:openFile:)`.
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn __application_did_finish_launching(&self, _notification: &NSNotification) {
            self.ivars().data.application_did_finish_launching(self);
        }
    }
);

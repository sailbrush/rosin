use std::sync::OnceLock;
use std::{cell::RefCell, rc::Rc};

use pollster::FutureExt;

use crate::{platform::view::RosinView, prelude::*, vello, wgpu};

static APP_STARTED: OnceLock<()> = OnceLock::new();

pub(crate) struct AppLauncher<S: Sync + 'static> {
    windows: Vec<WindowDesc<S>>,
    translation_map: Option<TranslationMap>,
    wgpu_config: WgpuConfig,
    _state: Option<Rc<RefCell<S>>>,
}

impl<S: Sync + 'static> AppLauncher<S> {
    pub fn new(window: WindowDesc<S>) -> Self {
        Self {
            windows: vec![window],
            translation_map: None,
            wgpu_config: WgpuConfig::default(),
            _state: None,
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

    pub fn run(self, _state: S, _translation_map: TranslationMap) -> Result<(), LaunchError> {
        self.run_impl(_state, _translation_map)
    }

    fn run_impl(mut self, state: S, translation_map: TranslationMap) -> Result<(), LaunchError> {
        if APP_STARTED.set(()).is_err() {
            return Err(LaunchError::AlreadyStarted);
        }

        use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, TranslateMessage};

        println!("Running {}", self.windows.first().unwrap().title.as_deref().unwrap_or("rosin app")); // TODO remove

        let instance = unsafe {
            // TODO: failure to create a window should **not** cause a crash
            // doing it this way so it's not ignored
            windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                .unwrap_or_else(|err| panic!("Failed to get windows instance: \"{err}\""))
                .into()
        };

        let wc = windows::Win32::UI::WindowsAndMessaging::WNDCLASSW {
            lpszClassName: crate::platform::proc_fn::ROSIN_CLASS,
            lpfnWndProc: Some(crate::platform::proc_fn::proc),
            hInstance: instance,
            ..Default::default()
        };

        let _class_atom = unsafe { windows::Win32::UI::WindowsAndMessaging::RegisterClassW(&raw const wc) };

        let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = match wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: self.wgpu_config.power_preference,
                force_fallback_adapter: false,
                compatible_surface: None,
                apply_limit_buckets: false,
            })
            .block_on()
        {
            Ok(adapter) => adapter,
            Err(e) => todo!("Failed to request a WGPU adapter; err info: {e:?}"),
        };

        if let Some(on_adapter_init) = self.wgpu_config.on_adapter_init {
            on_adapter_init(&adapter);
        }

        println!("GPU: {}", adapter.get_info().name);

        let (device, queue) = match adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("RosinDevice"),
                required_features: self.wgpu_config.features,
                required_limits: self.wgpu_config.limits.clone(),
                memory_hints: self.wgpu_config.memory_hints.clone(),
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .block_on()
        {
            Ok((device, queue)) => (device, queue),
            Err(e) => todo!("Failed to create a WGPU device; err info: {e:?}"),
        };

        if let Some(on_device_init) = self.wgpu_config.on_device_init {
            on_device_init(&device);
        }

        let compositor = Compositor {
            blitter: RefCell::new(None),
            custom: RefCell::new(None),
        };

        let vello_renderer = {
            let renderer = match vello::Renderer::new(
                &device,
                vello::RendererOptions {
                    use_cpu: false,
                    antialiasing_support: vello::AaSupport::all(),
                    num_init_threads: None,
                    pipeline_cache: None,
                },
            ) {
                Ok(r) => r,
                Err(e) => todo!("failed to create vello renderer; error info: {e:?}"),
            };

            Rc::new(RefCell::new(renderer))
        };

        let gpu_ctx = Rc::new(GpuCtx {
            instance: wgpu_instance,
            adapter,
            device,
            queue,
            compositor,
        });

        self._state = Some(Rc::new(RefCell::new(state)));
        self.translation_map = Some(translation_map);

        // temp fix
        let translation_map_ref = self.translation_map.as_ref().expect("We set it to Some on the line above");

        let window_handles: Vec<_> = self
            .windows
            .iter()
            .map(|window_desc| {
                RosinView::from_new_window(
                    window_desc,
                    Some(instance),
                    None,
                    translation_map_ref,
                    self._state.clone().unwrap(),
                    gpu_ctx.clone(),
                    vello_renderer.clone(),
                )
                .unwrap_or_else(|err| todo!("Failed to create window: \"{err}\""))
            })
            .map(|view| {
                let platformhandle = crate::platform::handle::WindowHandle::new(view);
                let rosinhandle = crate::handle::WindowHandle(platformhandle.clone());

                let state = platformhandle.view.try_on_thread(|view| {
                    return view.get_view_state();
                });

                let mut nonnull = state.unwrap().unwrap();

                let viewstate = unsafe { nonnull.as_mut() };

                viewstate.handle = Some(rosinhandle.clone());
                viewstate.viewport.init_viewport(&rosinhandle);

                platformhandle
            })
            .collect();

        #[cfg(debug_assertions)]
        if let Some(_handle) = window_handles.first() {
            //test windowhandles here
        }

        let mut message = MSG::default();

        #[cfg(false)]
        println!("\nStarting Rosin Loop...");

        // TODO decide: how threading works here (since windows *can* come from diferent threads)
        loop {
            #[cfg(false)]
            println!("\nInitializing new message...");

            let result = unsafe { GetMessageW(&raw mut message, None, 0, 0) };

            #[cfg(false)]
            println!("{result:?}; {message:?}");

            if result.0 <= 0 {
                #[cfg(false)]
                eprintln!("WARNING: The current implementation may stop the loop and may stop the program once one window closes");
                match result.0 {
                    0 => println!("\nStopping Rosin Loop..."),
                    code => {
                        let err = windows::core::Error::from_thread();
                        println!("Crashing Rosin Loop `{code:4X}` \"{err}\":\n - {err:#?}")
                    }
                }
                break;
            }

            unsafe {
                // TODO decide: ignore or `continue`?
                #[expect(unused_must_use)]
                TranslateMessage(&raw mut message);
                let _ = DispatchMessageW(&raw mut message);
            }
        }

        Ok(())
    }
}
